//! 自定义提取器
//!
//! 集成 Chimera 依赖注入的提取器

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use chimera_core::prelude::*;
use std::sync::Arc;
use regex::Regex;

/// Bean 提取器 - 从应用上下文中提取 Bean
///
/// 用法示例：
/// ```ignore
/// async fn handler(Bean(service): Bean<UserService>) -> impl IntoResponse {
///     service.do_something()
/// }
/// ```
pub struct Bean<T>(pub Arc<T>)
where
    T: Send + Sync + 'static;

#[async_trait]
impl<S, T> FromRequestParts<S> for Bean<T>
where
    Arc<ApplicationContext>: FromRef<S>,
    T: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = BeanExtractionError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let context = Arc::<ApplicationContext>::from_ref(state);

        let bean = context
            .get_bean_by_type::<T>()
            .await
            .map_err(|e| BeanExtractionError::NotFound(e.to_string()))?;

        Ok(Bean(bean))
    }
}

/// Bean 提取错误
#[derive(Debug)]
pub enum BeanExtractionError {
    NotFound(String),
}

impl IntoResponse for BeanExtractionError {
    fn into_response(self) -> Response {
        match self {
            BeanExtractionError::NotFound(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Bean not found: {}", msg))
                    .into_response()
            }
        }
    }
}

/// 可选 Bean 提取器 - 如果 Bean 不存在则返回 None
///
/// 用法示例：
/// ```ignore
/// async fn handler(OptionalBean(service): OptionalBean<MetricsService>) -> impl IntoResponse {
///     if let Some(svc) = service {
///         svc.track("request");
///     }
/// }
/// ```
pub struct OptionalBean<T>(pub Option<Arc<T>>)
where
    T: Send + Sync + 'static;

#[async_trait]
impl<S, T> FromRequestParts<S> for OptionalBean<T>
where
    Arc<ApplicationContext>: FromRef<S>,
    T: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let context = Arc::<ApplicationContext>::from_ref(state);

        let bean = context.get_bean_by_type::<T>().await.ok();

        Ok(OptionalBean(bean))
    }
}

/// 带正则验证的路径参数提取器
///
/// 如果参数不匹配正则，返回 404（路由不存在）
pub struct ValidatedPath<T> {
    pub inner: T,
}

impl<T> ValidatedPath<T> {
    /// 验证单个字符串参数
    pub fn validate_single(value: String, pattern: &str) -> Result<String, PathValidationError> {
        let re = Regex::new(pattern)
            .map_err(|_| PathValidationError::InvalidPattern)?;

        if re.is_match(&value) {
            Ok(value)
        } else {
            Err(PathValidationError::ValidationFailed)
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for ValidatedPath<String>
where
    S: Send + Sync,
{
    type Rejection = PathValidationError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value): Path<String> = Path::from_request_parts(parts, state)
            .await
            .map_err(|_| PathValidationError::ExtractionFailed)?;

        Ok(ValidatedPath { inner: value })
    }
}

/// 路径验证错误
#[derive(Debug)]
pub enum PathValidationError {
    InvalidPattern,
    ValidationFailed,
    ExtractionFailed,
}

impl IntoResponse for PathValidationError {
    fn into_response(self) -> Response {
        // 验证失败返回 404，表示路由不存在
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

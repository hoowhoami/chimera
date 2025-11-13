//! 自定义提取器
//!
//! 集成 Chimera 依赖注入的提取器

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use chimera_core::prelude::*;
use std::sync::Arc;

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

//! 自定义提取器
//!
//! 集成 Chimera 依赖注入的提取器

use axum::{
    async_trait,
    extract::{FromRequest, FromRequestParts, Path, Request},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json, Extension,
};
use chimera_core::prelude::*;
use std::sync::Arc;
use regex::Regex;
use serde::de::DeserializeOwned;

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

/// Autowired 提取器 - 类似 Spring Boot 的 @Autowired
///
/// 这与 Bean 功能完全相同，提供更符合 Spring Boot 习惯的命名
///
/// 用法示例：
/// ```ignore
/// async fn handler(Autowired(service): Autowired<UserService>) -> impl IntoResponse {
///     service.do_something()
/// }
/// ```
pub struct Autowired<T>(pub Arc<T>)
where
    T: Send + Sync + 'static;

#[async_trait]
impl<S, T> FromRequestParts<S> for Bean<T>
where
    T: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = BeanExtractionError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(context) = Extension::<Arc<ApplicationContext>>::from_request_parts(parts, state)
            .await
            .map_err(|_| BeanExtractionError::NotFound("ApplicationContext not found in extensions".to_string()))?;

        let bean = context
            .get_bean_by_type::<T>()
            .await
            .map_err(|e| BeanExtractionError::NotFound(e.to_string()))?;

        Ok(Bean(bean))
    }
}

#[async_trait]
impl<S, T> FromRequestParts<S> for Autowired<T>
where
    T: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = BeanExtractionError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(context) = Extension::<Arc<ApplicationContext>>::from_request_parts(parts, state)
            .await
            .map_err(|_| BeanExtractionError::NotFound("ApplicationContext not found in extensions".to_string()))?;

        let bean = context
            .get_bean_by_type::<T>()
            .await
            .map_err(|e| BeanExtractionError::NotFound(e.to_string()))?;

        Ok(Autowired(bean))
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
    T: Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let context = Extension::<Arc<ApplicationContext>>::from_request_parts(parts, state)
            .await
            .ok()
            .map(|Extension(ctx)| ctx);

        let bean = match context {
            Some(ctx) => ctx.get_bean_by_type::<T>().await.ok(),
            None => None,
        };

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

/// RequestBody 提取器 - 类似 Spring Boot 的 @RequestBody
///
/// 自动从 JSON 请求体中反序列化对象
///
/// 用法示例：
/// ```ignore
/// #[post_mapping("/users")]
/// async fn create_user(&self, RequestBody(user): RequestBody<CreateUserRequest>) -> impl IntoResponse {
///     // user 已经从 JSON 反序列化
///     ResponseEntity::created(user)
/// }
/// ```
pub struct RequestBody<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for RequestBody<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = RequestBodyError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| RequestBodyError::JsonParseError(e.to_string()))?;

        Ok(RequestBody(value))
    }
}

/// RequestBody 提取错误
#[derive(Debug)]
pub enum RequestBodyError {
    JsonParseError(String),
}

impl IntoResponse for RequestBodyError {
    fn into_response(self) -> Response {
        match self {
            RequestBodyError::JsonParseError(msg) => {
                (StatusCode::BAD_REQUEST, format!("Invalid request body: {}", msg))
                    .into_response()
            }
        }
    }
}

/// RequestParam 提取器 - 类似 Spring Boot 的 @RequestParam
///
/// 自动从 Query 参数中提取和反序列化对象
///
/// 用法示例：
/// ```ignore
/// #[get_mapping("/users/search")]
/// async fn search_users(&self, RequestParam(params): RequestParam<SearchQuery>) -> impl IntoResponse {
///     // params 已经从 query 参数反序列化
///     ResponseEntity::ok(results)
/// }
/// ```
pub struct RequestParam<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for RequestParam<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = RequestParamError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(value) = axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| RequestParamError::ParseError(e.to_string()))?;

        Ok(RequestParam(value))
    }
}

/// RequestParam 提取错误
#[derive(Debug)]
pub enum RequestParamError {
    ParseError(String),
}

impl IntoResponse for RequestParamError {
    fn into_response(self) -> Response {
        match self {
            RequestParamError::ParseError(msg) => {
                (StatusCode::BAD_REQUEST, format!("Invalid query parameters: {}", msg))
                    .into_response()
            }
        }
    }
}

/// PathVariable 提取器 - 类似 Spring Boot 的 @PathVariable
///
/// 自动从路径参数中提取值（这是对 Axum Path 的语义化封装）
///
/// 用法示例：
/// ```ignore
/// #[get_mapping("/users/:id")]
/// async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
///     // id 已经从路径参数提取并解析
///     ResponseEntity::ok(user)
/// }
/// ```
pub struct PathVariable<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for PathVariable<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = PathVariableError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| PathVariableError::ParseError(e.to_string()))?;

        Ok(PathVariable(value))
    }
}

/// PathVariable 提取错误
#[derive(Debug)]
pub enum PathVariableError {
    ParseError(String),
}

impl IntoResponse for PathVariableError {
    fn into_response(self) -> Response {
        match self {
            PathVariableError::ParseError(msg) => {
                (StatusCode::BAD_REQUEST, format!("Invalid path parameter: {}", msg))
                    .into_response()
            }
        }
    }
}

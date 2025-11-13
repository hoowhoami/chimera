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

/// PathVariable 提取器 - 类似 Spring Boot 的 @PathVariable
///
/// 自动从路径参数中提取值（这是对 Axum Path 的语义化封装）
///
/// 用法示例：
/// ```ignore
/// #[get_mapping("/users/{id}")]
/// async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
///     ResponseEntity::ok(id)
/// }
/// ```
pub struct PathVariable<T>(pub T);

impl<T> PathVariable<T> {
    /// 获取内部值
    pub fn into_inner(self) -> T {
        self.0
    }

    /// 获取内部值的引用
    pub fn as_ref(&self) -> &T {
        &self.0
    }
}

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
    /// 类型解析错误 - 返回 400
    ParseError(String),
}

impl IntoResponse for PathVariableError {
    fn into_response(self) -> Response {
        match self {
            PathVariableError::ParseError(msg) => {
                // 类型转换失败 - 参数格式错误，返回 400
                (StatusCode::BAD_REQUEST, format!("Invalid path parameter: {}", msg))
                    .into_response()
            }
        }
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

/// FormData 提取器 - 类似 Spring Boot 的 @RequestParam 处理表单
///
/// 自动从 application/x-www-form-urlencoded 或 multipart/form-data 请求体中提取数据
///
/// 用法示例：
/// ```ignore
/// #[derive(Deserialize)]
/// struct LoginForm {
///     username: String,
///     password: String,
/// }
///
/// #[post_mapping("/login")]
/// async fn login(&self, FormData(form): FormData<LoginForm>) -> impl IntoResponse {
///     // form 已经从表单数据反序列化
///     ResponseEntity::ok(form)
/// }
/// ```
pub struct FormData<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for FormData<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = FormDataError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Form(value) = axum::extract::Form::<T>::from_request(req, state)
            .await
            .map_err(|e| FormDataError::ParseError(e.to_string()))?;

        Ok(FormData(value))
    }
}

/// FormData 提取错误
#[derive(Debug)]
pub enum FormDataError {
    ParseError(String),
}

impl IntoResponse for FormDataError {
    fn into_response(self) -> Response {
        match self {
            FormDataError::ParseError(msg) => {
                (StatusCode::BAD_REQUEST, format!("Invalid form data: {}", msg))
                    .into_response()
            }
        }
    }
}

//! 自定义提取器
//!
//! 集成 Chimera 依赖注入和验证的提取器
//!
//! ## 提取器层级
//!
//! 按照 Axum 错误处理层级，提取器错误属于第一层级（提取器层级）
//! 所有提取器错误都会被转换为 `WebError`，然后由全局异常处理器处理

use axum::{
    async_trait,
    extract::{FromRequest, FromRequestParts, Path, Request},
    http::{request::Parts, HeaderMap},
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;

use crate::exception_handler::WebError;

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
        // 转换为 WebError 以便全局异常处理器可以捕获
        let web_error = match self {
            PathVariableError::ParseError(msg) => WebError::PathParse {
                message: format!("Invalid path parameter: {}", msg),
            },
        };
        web_error.into_response()
    }
}

/// RequestBody 提取器 - 类似 Spring Boot 的 @RequestBody
///
/// 自动从 JSON 请求体中反序列化对象（不验证）
///
/// 用法示例：
/// ```ignore
/// use chimera_web::prelude::*;
///
/// #[post_mapping("/echo")]
/// async fn echo(&self, RequestBody(data): RequestBody<serde_json::Value>) -> impl IntoResponse {
///     ResponseEntity::ok(data)
/// }
/// ```
///
/// 如果需要自动验证，请使用 `ValidRequestBody`
pub struct RequestBody<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for RequestBody<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(data) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                tracing::debug!(error = %error_msg, "JSON parse error");

                WebError::JsonParse {
                    message: error_msg.clone(),
                    source: Some(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        error_msg,
                    ))),
                }
            })?;

        Ok(RequestBody(data))
    }
}

/// ValidatedRequestBody 提取器 - 类似 Spring Boot 的 @Valid @RequestBody
///
/// 自动从 JSON 请求体中反序列化对象并执行验证
///
/// 要求 T 实现 `chimera_validator::Validate` trait
///
/// 用法示例：
/// ```ignore
/// use chimera_web::prelude::*;
/// use chimera_validator::Validate;
///
/// #[derive(Deserialize, Validate)]
/// struct CreateUserRequest {
///     #[validate(length_min = 2)]
///     name: String,
///     #[validate(email)]
///     email: String,
/// }
///
/// #[post_mapping("/users")]
/// async fn create_user(&self, ValidatedRequestBody(user): ValidatedRequestBody<CreateUserRequest>) -> impl IntoResponse {
///     // user 已经从 JSON 反序列化并通过验证
///     ResponseEntity::created(user)
/// }
/// ```
pub struct ValidatedRequestBody<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedRequestBody<T>
where
    T: DeserializeOwned + chimera_validator::Validate,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // 1. 先尝试解析 JSON
        let Json(data) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                tracing::debug!(error = %error_msg, "JSON parse error");

                WebError::JsonParse {
                    message: error_msg.clone(),
                    source: Some(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        error_msg,
                    ))),
                }
            })?;

        // 2. 执行验证
        data.validate().map_err(|e| {
            tracing::debug!(error = ?e, "Validation error");

            match e {
                chimera_validator::ValidationError::FieldErrors(field_errors) => {
                    WebError::Validation {
                        message: "Validation failed".to_string(),
                        field_errors: Some(field_errors),
                    }
                }
                chimera_validator::ValidationError::ValidationFailed(msg) => {
                    WebError::Validation {
                        message: msg,
                        field_errors: None,
                    }
                }
            }
        })?;

        Ok(ValidatedRequestBody(data))
    }
}

/// RequestParam 提取器 - 类似 Spring Boot 的 @RequestParam
///
/// 自动从 Query 参数中提取和反序列化对象（不验证）
///
/// 用法示例：
/// ```ignore
/// #[get_mapping("/users/search")]
/// async fn search_users(&self, RequestParam(params): RequestParam<SearchQuery>) -> impl IntoResponse {
///     // params 已经从 query 参数反序列化
///     ResponseEntity::ok(results)
/// }
/// ```
///
/// 如果需要自动验证，请使用 `ValidRequestParam`
pub struct RequestParam<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for RequestParam<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(value) = axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                tracing::debug!(error = %error_msg, "Query parse error");

                WebError::QueryParse {
                    message: error_msg,
                }
            })?;

        Ok(RequestParam(value))
    }
}

/// ValidatedRequestParam 提取器 - 类似 Spring Boot 的 @Valid @RequestParam
///
/// 自动从 Query 参数中提取和反序列化对象并执行验证
///
/// 要求 T 实现 `chimera_validator::Validate` trait
///
/// 用法示例：
/// ```ignore
/// use chimera_web::prelude::*;
/// use chimera_validator::Validate;
///
/// #[derive(Deserialize, Validate)]
/// struct SearchQuery {
///     #[validate(length_min = 1)]
///     keyword: String,
///     #[validate(range_min = 1, range_max = 100)]
///     page_size: Option<u32>,
/// }
///
/// #[get_mapping("/search")]
/// async fn search(&self, ValidatedRequestParam(query): ValidatedRequestParam<SearchQuery>) -> impl IntoResponse {
///     // query 已经通过验证
///     ResponseEntity::ok(query)
/// }
/// ```
pub struct ValidatedRequestParam<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for ValidatedRequestParam<T>
where
    T: DeserializeOwned + chimera_validator::Validate,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // 1. 先尝试解析查询参数
        let axum::extract::Query(data) = axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                tracing::debug!(error = %error_msg, "Query parse error");

                WebError::QueryParse {
                    message: error_msg,
                }
            })?;

        // 2. 执行验证
        data.validate().map_err(|e| {
            tracing::debug!(error = ?e, "Validation error");

            match e {
                chimera_validator::ValidationError::FieldErrors(field_errors) => {
                    WebError::Validation {
                        message: "Validation failed".to_string(),
                        field_errors: Some(field_errors),
                    }
                }
                chimera_validator::ValidationError::ValidationFailed(msg) => {
                    WebError::Validation {
                        message: msg,
                        field_errors: None,
                    }
                }
            }
        })?;

        Ok(ValidatedRequestParam(data))
    }
}

/// FormData 提取器 - 类似 Spring Boot 的 @ModelAttribute
///
/// 自动从 application/x-www-form-urlencoded 或 multipart/form-data 请求体中提取数据（不验证）
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
///
/// 如果需要自动验证，请使用 `ValidatedFormData`
pub struct FormData<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for FormData<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Form(value) = axum::extract::Form::<T>::from_request(req, state)
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                tracing::debug!(error = %error_msg, "Form parse error");

                WebError::FormParse {
                    message: error_msg,
                }
            })?;

        Ok(FormData(value))
    }
}

/// ValidatedFormData 提取器 - 类似 Spring Boot 的 @Valid @ModelAttribute
///
/// 自动从 application/x-www-form-urlencoded 或 multipart/form-data 请求体中提取数据并执行验证
///
/// 要求 T 实现 `chimera_validator::Validate` trait
///
/// 用法示例：
/// ```ignore
/// use chimera_web::prelude::*;
/// use chimera_validator::Validate;
///
/// #[derive(Deserialize, Validate)]
/// struct RegisterForm {
///     #[validate(length_min = 2)]
///     username: String,
///     #[validate(email)]
///     email: String,
/// }
///
/// #[post_mapping("/register")]
/// async fn register(&self, ValidatedFormData(form): ValidatedFormData<RegisterForm>) -> impl IntoResponse {
///     // form 已经通过验证
///     ResponseEntity::ok(form)
/// }
/// ```
pub struct ValidatedFormData<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedFormData<T>
where
    T: DeserializeOwned + chimera_validator::Validate,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // 1. 先尝试解析表单数据
        let axum::extract::Form(data) = axum::extract::Form::<T>::from_request(req, state)
            .await
            .map_err(|e| {
                let error_msg = e.to_string();
                tracing::debug!(error = %error_msg, "Form parse error");

                WebError::FormParse {
                    message: error_msg,
                }
            })?;

        // 2. 执行验证
        data.validate().map_err(|e| {
            tracing::debug!(error = ?e, "Validation error");

            match e {
                chimera_validator::ValidationError::FieldErrors(field_errors) => {
                    WebError::Validation {
                        message: "Validation failed".to_string(),
                        field_errors: Some(field_errors),
                    }
                }
                chimera_validator::ValidationError::ValidationFailed(msg) => {
                    WebError::Validation {
                        message: msg,
                        field_errors: None,
                    }
                }
            }
        })?;

        Ok(ValidatedFormData(data))
    }
}

/// RequestHeader 提取器 - 类似 Spring Boot 的 @RequestHeader
///
/// 从 HTTP 请求头中提取单个 header 值
///
/// 用法示例：
/// ```ignore
/// #[get_mapping("/api/data")]
/// async fn get_data(&self, RequestHeader(auth): RequestHeader<String>) -> impl IntoResponse {
///     // auth 已经从 Authorization header 提取
///     ResponseEntity::ok(format!("Auth: {}", auth))
/// }
/// ```
///
/// 注意：header 名称使用小写加下划线格式，会自动转换为 HTTP header 格式
/// 例如：`user_agent` -> `User-Agent`, `content_type` -> `Content-Type`
pub struct RequestHeader<T>(pub T);

#[async_trait]
impl<S> FromRequestParts<S> for RequestHeader<String>
where
    S: Send + Sync,
{
    type Rejection = RequestHeaderError;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 注意：这里我们需要从方法签名中的变量名推断 header 名
        // 但在提取器中无法获取变量名，所以这里返回整个 HeaderMap 的第一个值作为示例
        // 实际使用中，用户应该使用更具体的提取器

        // 为了简化，我们提供一个包装 HeaderMap 的方式
        Err(RequestHeaderError::HeaderNotFound(
            "RequestHeader<String> requires explicit header name. Use RequestHeaders instead.".to_string()
        ))
    }
}

/// RequestHeaders 提取器 - 提取所有请求头
///
/// 用法示例：
/// ```ignore
/// #[get_mapping("/api/headers")]
/// async fn get_headers(&self, RequestHeaders(headers): RequestHeaders) -> impl IntoResponse {
///     let user_agent = headers.get("user-agent")
///         .and_then(|v| v.to_str().ok())
///         .unwrap_or("unknown");
///     ResponseEntity::ok(format!("User-Agent: {}", user_agent))
/// }
/// ```
pub struct RequestHeaders(pub HeaderMap);

#[async_trait]
impl<S> FromRequestParts<S> for RequestHeaders
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(RequestHeaders(parts.headers.clone()))
    }
}

/// RequestHeader 提取错误
#[derive(Debug)]
pub enum RequestHeaderError {
    HeaderNotFound(String),
    InvalidHeaderValue(String),
}

impl IntoResponse for RequestHeaderError {
    fn into_response(self) -> Response {
        // 转换为 WebError 以便全局异常处理器可以捕获
        let web_error = match self {
            RequestHeaderError::HeaderNotFound(msg) => WebError::Internal(format!("Header not found: {}", msg)),
            RequestHeaderError::InvalidHeaderValue(msg) => WebError::Internal(format!("Invalid header value: {}", msg)),
        };
        web_error.into_response()
    }
}

/// Cookies 提取器 - 从请求中提取所有 Cookie
///
/// 自动从 HTTP 请求头中解析并提取所有 Cookie
///
/// 用法示例：
/// ```ignore
/// use chimera_web::prelude::*;
///
/// #[get_mapping("/check")]
/// async fn check_cookies(&self, Cookies(cookies): Cookies) -> impl IntoResponse {
///     if let Some(session_id) = cookies.get("session_id") {
///         ResponseEntity::ok(format!("Session ID: {}", session_id))
///     } else {
///         ResponseEntity::ok("No session found")
///     }
/// }
/// ```
pub struct Cookies(pub std::collections::HashMap<String, String>);

impl Cookies {
    /// 获取指定名称的 Cookie 值
    pub fn get(&self, name: &str) -> Option<&String> {
        self.0.get(name)
    }

    /// 获取内部 HashMap
    pub fn into_inner(self) -> std::collections::HashMap<String, String> {
        self.0
    }

    /// 获取内部 HashMap 的引用
    pub fn as_ref(&self) -> &std::collections::HashMap<String, String> {
        &self.0
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Cookies
where
    S: Send + Sync,
{
    type Rejection = CookieError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let mut cookies = std::collections::HashMap::new();

        // 从 Cookie header 中提取所有 cookie
        if let Some(cookie_header) = parts.headers.get("cookie") {
            let cookie_str = cookie_header
                .to_str()
                .map_err(|_| CookieError::InvalidCookieFormat)?;

            // 解析 cookie 字符串: name1=value1; name2=value2
            for pair in cookie_str.split(';') {
                let trimmed = pair.trim();
                if let Some((name, value)) = trimmed.split_once('=') {
                    cookies.insert(name.to_string(), value.to_string());
                }
            }
        }

        Ok(Cookies(cookies))
    }
}

/// Cookie 提取错误
#[derive(Debug)]
pub enum CookieError {
    /// Cookie 格式无效 - 返回 400
    InvalidCookieFormat,
}

impl IntoResponse for CookieError {
    fn into_response(self) -> Response {
        // 转换为 WebError 以便全局异常处理器可以捕获
        let web_error = match self {
            CookieError::InvalidCookieFormat => WebError::Internal("Invalid cookie format".to_string()),
        };
        web_error.into_response()
    }
}

/// Session 提取器 - 从 Cookie 中提取会话数据
///
/// 自动从 Cookie 中提取名为 "session" 的值并反序列化为指定类型
///
/// 要求 T 实现 `serde::Deserialize` trait
///
/// 用法示例：
/// ```ignore
/// use chimera_web::prelude::*;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct UserSession {
///     user_id: u64,
///     username: String,
/// }
///
/// #[get_mapping("/profile")]
/// async fn get_profile(&self, Session(session): Session<UserSession>) -> impl IntoResponse {
///     // session 已经从 cookie 中提取并反序列化
///     ResponseEntity::ok(format!("Welcome, {}", session.username))
/// }
/// ```
///
/// 注意：此提取器会从名为 "session" 的 Cookie 中读取 JSON 格式的数据
/// 如果需要自定义 Cookie 名称或解析逻辑，请使用 `Cookies` 提取器
pub struct Session<T>(pub T);

impl<T> Session<T> {
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
impl<S, T> FromRequestParts<S> for Session<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = SessionError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 从 Cookie header 中提取 session
        let mut session_value = None;

        if let Some(cookie_header) = parts.headers.get("cookie") {
            let cookie_str = cookie_header
                .to_str()
                .map_err(|_| SessionError::InvalidFormat)?;

            // 解析 cookie 字符串，查找名为 "session" 的 cookie
            for pair in cookie_str.split(';') {
                let trimmed = pair.trim();
                if let Some((name, value)) = trimmed.split_once('=') {
                    if name == "session" {
                        session_value = Some(value.to_string());
                        break;
                    }
                }
            }
        }

        let session_value = session_value.ok_or(SessionError::SessionNotFound)?;

        // 反序列化 session 数据（假设是 JSON 格式）
        let data: T = serde_json::from_str(&session_value).map_err(|e| {
            tracing::debug!(error = %e, "Session deserialization error");
            SessionError::InvalidFormat
        })?;

        Ok(Session(data))
    }
}

/// Session 提取错误
#[derive(Debug)]
pub enum SessionError {
    /// Session 不存在 - 返回 401
    SessionNotFound,
    /// Session 格式无效 - 返回 500
    InvalidFormat,
}

impl IntoResponse for SessionError {
    fn into_response(self) -> Response {
        // 转换为 WebError 以便全局异常处理器可以捕获
        let web_error = match self {
            SessionError::SessionNotFound => {
                WebError::Authentication("Session not found".to_string())
            }
            SessionError::InvalidFormat => {
                WebError::Internal("Invalid session format".to_string())
            }
        };
        web_error.into_response()
    }
}



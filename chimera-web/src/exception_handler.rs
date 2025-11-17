//! 全局异常处理模块
//!
//! 提供类似 Spring Boot @ControllerAdvice 的全局异常处理功能
//!
//! ## Axum 错误处理层级
//!
//! 1. **提取器层级** - 请求参数解析错误（JSON、Path、Query等）
//! 2. **中间件层级** - 认证、限流等中间件错误
//! 3. **业务逻辑层级** - Handler 函数内的业务错误
//! 4. **全局处理层级** - 统一捕获和转换所有错误
//! 5. **框架底层层级** - HTTP 服务器、连接等底层错误

use async_trait::async_trait;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chimera_core::{ApplicationContext, Container};
use serde_json::Value;
use std::sync::Arc;
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// 🔥 Web 层错误类型 - 分层设计
// ============================================================================

/// Web 层错误类型
///
/// 按照 Axum 错误处理层级设计，只包含 Web 层的错误：
/// 1. **提取器层级** - JSON、Path、Query 等解析错误
/// 2. **中间件层级** - 认证、限流等中间件错误
/// 3. **框架底层** - HTTP 服务器、连接等底层错误
///
/// **注意**：业务逻辑错误由用户自己定义，通过实现 `std::error::Error` 和 `IntoResponse` 即可
#[derive(Error, Debug)]
pub enum WebError {
    // ========== 1. 提取器层级错误 ==========
    /// JSON 解析错误 - 400 Bad Request
    #[error("JSON parse error: {message}")]
    JsonParse {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// 参数验证错误 - 400 Bad Request
    #[error("Validation failed: {message}")]
    Validation {
        message: String,
        /// 字段级别的验证错误详情
        field_errors: Option<HashMap<String, Vec<String>>>,
    },

    /// 路径参数解析错误 - 400 Bad Request
    #[error("Invalid path parameter: {message}")]
    PathParse { message: String },

    /// 查询参数解析错误 - 400 Bad Request
    #[error("Invalid query parameter: {message}")]
    QueryParse { message: String },

    /// 表单数据解析错误 - 400 Bad Request
    #[error("Invalid form data: {message}")]
    FormParse { message: String },

    // ========== 2. 中间件层级错误 ==========
    /// 认证失败 - 401 Unauthorized
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// 授权失败 - 403 Forbidden
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// 限流错误 - 429 Too Many Requests
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    // ========== 3. 框架底层错误 ==========
    /// 内部服务器错误 - 500 Internal Server Error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// 包装用户自定义的业务错误
    ///
    /// 用户的业务错误需要实现 `std::error::Error + Send + Sync + 'static`
    /// 框架会通过全局异常处理器来处理这些错误
    #[error("Business error: {0}")]
    UserDefined(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl WebError {
    /// 获取错误对应的 HTTP 状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 提取器层级 - 400 Bad Request
            WebError::JsonParse { .. } => StatusCode::BAD_REQUEST,
            WebError::Validation { .. } => StatusCode::BAD_REQUEST,
            WebError::PathParse { .. } => StatusCode::BAD_REQUEST,
            WebError::QueryParse { .. } => StatusCode::BAD_REQUEST,
            WebError::FormParse { .. } => StatusCode::BAD_REQUEST,

            // 中间件层级
            WebError::Authentication(_) => StatusCode::UNAUTHORIZED,
            WebError::Authorization(_) => StatusCode::FORBIDDEN,
            WebError::RateLimit(_) => StatusCode::TOO_MANY_REQUESTS,

            // 框架底层
            WebError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,

            // 用户自定义错误 - 默认返回 500，用户可以通过全局异常处理器自定义
            WebError::UserDefined(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 获取错误详情（用于 JSON 响应）
    pub fn details(&self) -> Option<Value> {
        match self {
            WebError::Validation { field_errors, .. } => {
                field_errors.as_ref().map(|errors| serde_json::to_value(errors).unwrap())
            }
            _ => None,
        }
    }
}

/// 实现 IntoResponse，使 WebError 可以直接作为 Handler 返回值
///
/// 注意：这个实现会将 WebError 存储在响应的 Extension 中，
/// 以便全局异常处理中间件可以提取并使用自定义的异常处理器
impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let status = self.status_code();

        // 创建一个简单的错误响应
        let error_response = ErrorResponse {
            timestamp: chrono::Utc::now().to_rfc3339(),
            status: status.as_u16(),
            error: status.canonical_reason().unwrap_or("Unknown Error").to_string(),
            message: self.to_string(),
            path: "unknown".to_string(), // 在中间件中会被替换
            trace: None,
            details: self.details(),
        };

        // 将 WebError 存储在响应的 Extension 中，供中间件使用
        let mut response = (status, Json(error_response)).into_response();
        response.extensions_mut().insert(Arc::new(self));
        response
    }
}

/// 全局异常处理器 trait - 类似 Spring 的 @ControllerAdvice
///
/// 用户可以实现此 trait 来自定义异常处理逻辑
///
/// # 示例
///
/// ```ignore
/// use chimera_web::prelude::*;
///
/// #[derive(Component)]
/// pub struct MyExceptionHandler;
///
/// #[async_trait]
/// impl GlobalExceptionHandler for MyExceptionHandler {
///     fn name(&self) -> &str {
///         "MyExceptionHandler"
///     }
///
///     fn can_handle(&self, error: &WebError) -> bool {
///         // 判断是否可以处理该错误
///         matches!(error, WebError::UserDefined(_))
///     }
///
///     async fn handle_error(
///         &self,
///         error: &WebError,
///         request_path: &str,
///     ) -> Option<ErrorResponse> {
///         // 自定义错误处理逻辑
///         Some(ErrorResponse::new(
///             StatusCode::BAD_REQUEST,
///             "Custom Error".to_string(),
///             error.to_string(),
///             request_path.to_string(),
///         ))
///     }
/// }
/// ```
#[async_trait]
pub trait GlobalExceptionHandler: Send + Sync {
    fn name(&self) -> &str;

    /// 优先级，数字越小优先级越高
    fn priority(&self) -> i32 {
        100
    }

    /// 处理特定类型的异常
    ///
    /// 返回 `Some(ErrorResponse)` 表示已处理，返回 `None` 表示不处理
    async fn handle_error(
        &self,
        error: &WebError,
        request_path: &str,
    ) -> Option<ErrorResponse>;

    /// 判断是否可以处理该异常类型
    fn can_handle(&self, error: &WebError) -> bool;
}

/// 标准错误响应格式
#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub timestamp: String,
    pub status: u16,
    pub error: String,
    pub message: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>, // 开发环境显示堆栈
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>, // 额外错误详情
}

impl ErrorResponse {
    pub fn new(status: StatusCode, error: String, message: String, path: String) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            status: status.as_u16(),
            error,
            message,
            path,
            trace: None,
            details: None,
        }
    }

    pub fn with_trace(mut self, trace: String) -> Self {
        self.trace = Some(trace);
        self
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

// ============================================================================
// 🔥 全局异常处理器注册表
// ============================================================================

/// 异常处理器注册表
pub struct GlobalExceptionHandlerRegistry {
    handlers: Vec<Arc<dyn GlobalExceptionHandler>>,
}

impl GlobalExceptionHandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn register<H: GlobalExceptionHandler + 'static>(&mut self, handler: H) {
        self.handlers.push(Arc::new(handler));
        // 按优先级排序
        self.handlers.sort_by_key(|h| h.priority());
    }

    pub fn register_arc(&mut self, handler: Arc<dyn GlobalExceptionHandler>) {
        self.handlers.push(handler);
        // 按优先级排序
        self.handlers.sort_by_key(|h| h.priority());
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// 处理异常，返回标准化的错误响应
    ///
    /// 处理流程：
    /// 1. 依次尝试用户注册的异常处理器
    /// 2. 如果没有处理器处理，使用框架默认处理
    pub async fn handle_error(
        &self,
        error: &WebError,
        request_path: &str,
    ) -> ErrorResponse {
        // 依次尝试各个处理器
        for handler in &self.handlers {
            if handler.can_handle(error) {
                if let Some(response) = handler.handle_error(error, request_path).await {
                    tracing::debug!(
                        handler = handler.name(),
                        error = %error,
                        "Error handled by custom handler"
                    );
                    return response;
                }
            }
        }

        // 默认处理器 - 框架提供的默认错误响应
        self.default_error_response(error, request_path)
    }

    /// 框架默认的错误响应
    fn default_error_response(
        &self,
        error: &WebError,
        request_path: &str,
    ) -> ErrorResponse {
        let status = error.status_code();

        tracing::error!(
            error = %error,
            path = request_path,
            status = %status.as_u16(),
            "Error handled by default handler"
        );

        ErrorResponse {
            timestamp: chrono::Utc::now().to_rfc3339(),
            status: status.as_u16(),
            error: status.canonical_reason().unwrap_or("Unknown Error").to_string(),
            message: error.to_string(),
            path: request_path.to_string(),
            trace: None,
            details: error.details(),
        }
    }
}

impl Default for GlobalExceptionHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 🔥 框架核心：自动发现并构建异常处理器注册表
pub async fn build_exception_handler_registry(
    context: &Arc<ApplicationContext>,
) -> chimera_core::ApplicationResult<GlobalExceptionHandlerRegistry> {
    // 使用 inventory 机制自动发现所有异常处理器
    crate::exception_handler_registry::build_exception_handler_registry_from_inventory(context).await
}

/// 🔥 框架扩展接口：允许框架自动注册新的异常处理器类型
/// 用户可以通过这个接口让框架自动发现自定义的异常处理器
impl GlobalExceptionHandlerRegistry {
    /// 尝试从容器中获取指定类型的异常处理器并自动注册
    pub async fn auto_register_type<T>(
        &mut self,
        context: &Arc<ApplicationContext>,
    ) -> chimera_core::ApplicationResult<bool>
    where
        T: GlobalExceptionHandler + Clone + 'static,
    {
        match context.get_bean_by_type::<T>().await {
            Ok(handler) => {
                let handler_name = handler.name().to_string();
                self.register((*handler).clone());
                tracing::info!("Auto-registered exception handler: {}", handler_name);
                Ok(true)
            }
            Err(_) => {
                // Bean不存在，这是正常的
                Ok(false)
            }
        }
    }

    /// 批量自动注册多个类型
    /// 这个方法由框架调用，用户也可以在需要时调用
    pub async fn auto_register_common_types(
        &mut self,
        _context: &Arc<ApplicationContext>,
    ) -> chimera_core::ApplicationResult<usize> {
        let initial_count = self.len();

        // 这里可以添加常见的异常处理器类型
        // 框架开发者可以在这里添加新的类型，或者用户可以调用auto_register_type

        // 示例：如果用户定义了BusinessExceptionHandler，框架会自动发现
        // self.auto_register_type::<BusinessExceptionHandler>(context).await?;

        // 注意：这些类型需要在编译时已知，所以用户需要在某处告诉框架
        // 哪些类型需要自动注册

        let discovered_count = self.len() - initial_count;
        Ok(discovered_count)
    }
}

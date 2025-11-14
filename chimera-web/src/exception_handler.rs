//! 全局异常处理模块
//!
//! 提供类似 Spring Boot @ControllerAdvice 的全局异常处理功能

use async_trait::async_trait;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chimera_core::{ApplicationContext, Container};
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;

/// 全局异常处理器 trait - 类似Spring的@ControllerAdvice
#[async_trait]
pub trait GlobalExceptionHandler: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32 {
        100
    } // 数字越小优先级越高

    /// 处理特定类型的异常
    async fn handle_error(
        &self,
        error: &(dyn std::error::Error + Send + Sync),
        request_path: &str,
    ) -> Option<ErrorResponse>;

    /// 判断是否可以处理该异常类型
    fn can_handle(&self, error: &(dyn std::error::Error + Send + Sync)) -> bool;
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

/// 应用级异常类型定义
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Business logic error: {0}")]
    BusinessError(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Unauthorized access: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Request timeout: {0}")]
    Timeout(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
}

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        let error_response = ErrorResponse::new(
            self.status_code(),
            self.error_type().to_string(),
            self.to_string(),
            "".to_string(),
        );
        (self.status_code(), Json(error_response)).into_response()
    }
}

impl ApplicationError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BusinessError(_) => StatusCode::BAD_REQUEST,
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            Self::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            Self::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            Self::BusinessError(_) => "Business Error",
            Self::ValidationError(_) => "Validation Error",
            Self::NotFound(_) => "Not Found",
            Self::Unauthorized(_) => "Unauthorized",
            Self::Forbidden(_) => "Forbidden",
            Self::DatabaseError(_) => "Database Error",
            Self::ExternalServiceError(_) => "External Service Error",
            Self::Timeout(_) => "Request Timeout",
            Self::RateLimitExceeded(_) => "Rate Limit Exceeded",
        }
    }
}

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
    pub async fn handle_error(
        &self,
        error: &(dyn std::error::Error + Send + Sync),
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

        // 默认处理器
        self.default_error_response(error, request_path)
    }

    fn default_error_response(
        &self,
        error: &(dyn std::error::Error + Send + Sync),
        request_path: &str,
    ) -> ErrorResponse {
        tracing::error!(error = %error, path = request_path, "Unhandled error");

        // 尝试转换为ApplicationError
        if let Some(source) = error.source() {
            if let Some(app_error) = source.downcast_ref::<ApplicationError>() {
                return ErrorResponse::new(
                    app_error.status_code(),
                    app_error.error_type().to_string(),
                    app_error.to_string(),
                    request_path.to_string(),
                );
            }
        }

        // 尝试直接转换（通过字符串匹配检测ApplicationError类型的特征）
        let error_str = error.to_string();
        if error_str.contains("ValidationError") || error_str.contains("BusinessError") {
            return ErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "Application Error".to_string(),
                error_str,
                request_path.to_string(),
            );
        }

        // 处理Axum内置的拒绝类型（通过错误信息检测）
        if error_str.contains("JsonRejection") || error_str.contains("Invalid JSON") {
            return ErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "Bad Request".to_string(),
                format!("Invalid JSON: {}", error_str),
                request_path.to_string(),
            );
        }

        if error_str.contains("PathRejection") || error_str.contains("Invalid path") {
            return ErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "Bad Request".to_string(),
                format!("Invalid path parameter: {}", error_str),
                request_path.to_string(),
            );
        }

        if error_str.contains("QueryRejection") || error_str.contains("Invalid query") {
            return ErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "Bad Request".to_string(),
                format!("Invalid query parameter: {}", error_str),
                request_path.to_string(),
            );
        }

        // 默认的500错误
        ErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
            format!("{}", error),
            request_path.to_string(),
        )
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

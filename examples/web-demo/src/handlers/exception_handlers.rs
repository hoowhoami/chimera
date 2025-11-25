//! 业务异常处理器示例
//!
//! 展示如何实现自定义的全局异常处理器

use chimera_core::Component;
use chimera_core_macros::Component;
use chimera_web::exception_handler::{ErrorResponse, GlobalExceptionHandler, WebError};
use chimera_web_macros::ExceptionHandler;
use serde_json::json;

use crate::error::BusinessError;

/// 业务异常处理器 - 类似Spring的@ControllerAdvice
/// 🔥 用户只需要添加这两个注解，框架自动完成注册！
#[derive(ExceptionHandler, Component)]
#[component("businessExceptionHandler")]
pub struct BusinessExceptionHandler {
    #[value("app.debug", default = false)]
    debug_mode: bool,
}

#[async_trait::async_trait]
impl GlobalExceptionHandler for BusinessExceptionHandler {
    fn name(&self) -> &str {
        "BusinessExceptionHandler"
    }

    fn priority(&self) -> i32 {
        10 // 高优先级，优先处理业务异常
    }

    fn can_handle(&self, error: &WebError) -> bool {
        // 检查是否是用户自定义的业务错误
        matches!(error, WebError::UserDefined(_))
    }

    async fn handle_error(
        &self,
        error: &WebError,
        request_path: &str,
    ) -> Option<ErrorResponse> {
        match error {
            WebError::UserDefined(e) => {
                // 尝试 downcast 到 BusinessError
                if let Some(business_error) = e.downcast_ref::<BusinessError>() {
                    let (status_code, error_type) = match business_error {
                        BusinessError::UserNotFound(_) => {
                            (axum::http::StatusCode::NOT_FOUND, "UserNotFound")
                        }
                        BusinessError::UserAlreadyExists(_) => {
                            (axum::http::StatusCode::CONFLICT, "UserAlreadyExists")
                        }
                        BusinessError::InvalidCredentials => {
                            (axum::http::StatusCode::UNAUTHORIZED, "InvalidCredentials")
                        }
                        BusinessError::InsufficientPermissions(_) => {
                            (axum::http::StatusCode::FORBIDDEN, "InsufficientPermissions")
                        }
                        BusinessError::ResourceNotFound(_) => {
                            (axum::http::StatusCode::NOT_FOUND, "ResourceNotFound")
                        }
                        BusinessError::DatabaseError(_) => {
                            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "DatabaseError")
                        }
                        BusinessError::ValidationError(_) => {
                            (axum::http::StatusCode::BAD_REQUEST, "ValidationError")
                        }
                    };

                    let mut response = ErrorResponse::new(
                        status_code,
                        error_type.to_string(),
                        business_error.to_string(),
                        request_path.to_string(),
                    );

                    // 在调试模式下添加堆栈信息
                    if self.debug_mode {
                        response = response.with_trace(format!("{:?}", business_error));
                    }

                    Some(response)
                } else {
                    // 其他用户自定义错误，使用通用处理
                    let mut response = ErrorResponse::new(
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "BusinessError".to_string(),
                        e.to_string(),
                        request_path.to_string(),
                    );

                    if self.debug_mode {
                        response = response.with_trace(format!("{:?}", e));
                    }

                    Some(response)
                }
            }
            _ => None, // 不处理其他类型的错误
        }
    }
}

/// 验证错误处理器 - 专门处理验证错误，提供更友好的错误信息
/// 🔥 用户只需要添加这两个注解，框架自动完成注册！
#[derive(ExceptionHandler, Component)]
#[component("validationExceptionHandler")]
pub struct ValidationExceptionHandler {
    #[value("app.debug", default = false)]
    debug_mode: bool,
}

#[async_trait::async_trait]
impl GlobalExceptionHandler for ValidationExceptionHandler {
    fn name(&self) -> &str {
        "ValidationExceptionHandler"
    }

    fn priority(&self) -> i32 {
        20 // 中等优先级
    }

    fn can_handle(&self, error: &WebError) -> bool {
        // 只处理验证错误
        matches!(error, WebError::Validation { .. })
    }

    async fn handle_error(
        &self,
        error: &WebError,
        request_path: &str,
    ) -> Option<ErrorResponse> {
        match error {
            WebError::Validation { message, field_errors } => {
                let mut response = ErrorResponse::new(
                    axum::http::StatusCode::BAD_REQUEST,
                    "ValidationError".to_string(),
                    message.clone(),
                    request_path.to_string(),
                );

                // 添加字段级别的验证错误详情
                if let Some(fields) = field_errors {
                    response = response.with_details(json!({
                        "field_errors": fields,
                        "error_type": "VALIDATION_ERROR",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }));
                }

                if self.debug_mode {
                    response = response.with_trace(format!("{:?}", error));
                }

                Some(response)
            }
            _ => None,
        }
    }
}


/// 默认异常处理器 - 处理所有未被其他处理器处理的错误
/// 🔥 用户只需要添加这两个注解，框架自动完成注册！
#[derive(ExceptionHandler, Component)]
#[component("defaultExceptionHandler")]
pub struct DefaultExceptionHandler {
    #[value("app.debug", default = true)]
    debug_mode: bool,
}

#[async_trait::async_trait]
impl GlobalExceptionHandler for DefaultExceptionHandler {

    fn name(&self) -> &str {
        "DefaultExceptionHandler"
    }

    fn priority(&self) -> i32 {
        100 // 最低优先级，作为兜底处理器
    }

    fn can_handle(&self, _error: &WebError) -> bool {
        true // 处理所有错误
    }

    async fn handle_error(
        &self,
        error: &WebError,
        request_path: &str,
    ) -> Option<ErrorResponse> {
        // 根据 WebError 类型返回不同的响应
        let (status_code, error_type, message) = match error {
            WebError::JsonParse { message, .. } => {
                (axum::http::StatusCode::BAD_REQUEST, "JsonParseError", message.clone())
            }
            WebError::Validation { message, .. } => {
                (axum::http::StatusCode::BAD_REQUEST, "ValidationError", message.clone())
            }
            WebError::PathParse { message } => {
                (axum::http::StatusCode::BAD_REQUEST, "PathParseError", message.clone())
            }
            WebError::QueryParse { message } => {
                (axum::http::StatusCode::BAD_REQUEST, "QueryParseError", message.clone())
            }
            WebError::FormParse { message } => {
                (axum::http::StatusCode::BAD_REQUEST, "FormParseError", message.clone())
            }
            WebError::Authentication(message) => {
                (axum::http::StatusCode::UNAUTHORIZED, "AuthenticationError", message.clone())
            }
            WebError::Authorization(message) => {
                (axum::http::StatusCode::FORBIDDEN, "AuthorizationError", message.clone())
            }
            WebError::RateLimit(message) => {
                (axum::http::StatusCode::TOO_MANY_REQUESTS, "RateLimitError", message.clone())
            }
            WebError::Internal(message) => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "InternalError",
                 if self.debug_mode { message.clone() } else { "Internal server error".to_string() })
            }
            WebError::UserDefined(e) => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "UserDefinedError",
                 if self.debug_mode { e.to_string() } else { "An error occurred".to_string() })
            }
        };

        let mut response = ErrorResponse::new(
            status_code,
            error_type.to_string(),
            message,
            request_path.to_string(),
        );

        if self.debug_mode {
            response = response.with_trace(format!("{:?}", error));
        }

        // 添加错误的详细信息
        if let Some(details) = error.details() {
            response = response.with_details(details);
        }

        Some(response)
    }

}
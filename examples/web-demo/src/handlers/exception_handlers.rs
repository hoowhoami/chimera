//! 业务异常处理器示例
//!
//! 展示如何实现自定义的全局异常处理器

use chimera_core::ComponentBean;
use chimera_core_macros::{bean, Component};
use chimera_web::exception_handler::{ErrorResponse, GlobalExceptionHandler};
use chimera_web_macros::ExceptionHandler;
use serde_json::{json, value};

/// 业务异常处理器 - 类似Spring的@ControllerAdvice
/// 🔥 用户只需要添加这两个注解，框架自动完成注册！
#[derive(ExceptionHandler, Component)]
#[bean("businessExceptionHandler")]
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

    fn can_handle(&self, error: &(dyn std::error::Error + Send + Sync)) -> bool {
        // 通过字符串匹配检查是否是ApplicationError类型
        let error_str = error.to_string();
        error_str.contains("Business logic error")
            || error_str.contains("Validation failed")
            || error_str.contains("Resource not found")
            || error_str.contains("Unauthorized access")
            || error_str.contains("Forbidden")
    }

    async fn handle_error(
        &self,
        error: &(dyn std::error::Error + Send + Sync),
        request_path: &str,
    ) -> Option<ErrorResponse> {
        let error_str = error.to_string();

        // 通过错误信息判断具体的ApplicationError类型
        let (status_code, error_type) = if error_str.contains("Validation failed") {
            (axum::http::StatusCode::BAD_REQUEST, "ValidationError")
        } else if error_str.contains("Business logic error") {
            (
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                "BusinessError",
            )
        } else if error_str.contains("Resource not found") {
            (axum::http::StatusCode::NOT_FOUND, "NotFound")
        } else if error_str.contains("Unauthorized access") {
            (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized")
        } else if error_str.contains("Forbidden") {
            (axum::http::StatusCode::FORBIDDEN, "Forbidden")
        } else {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "ApplicationError",
            )
        };

        let mut response = ErrorResponse::new(
            status_code,
            error_type.to_string(),
            error_str.clone(),
            request_path.to_string(),
        );

        // 在调试模式下添加堆栈信息
        if self.debug_mode {
            response = response.with_trace(format!("{:?}", error));
        }

        // 为验证错误添加详细信息
        if error_str.contains("Validation failed") {
            response = response.with_details(json!({
                "validation_failures": self.parse_validation_errors(&error_str)
            }));
        }

        Some(response)
    }
}

impl BusinessExceptionHandler {
    fn parse_validation_errors(&self, msg: &str) -> Vec<serde_json::Value> {
        // 这里可以实现更复杂的验证错误解析逻辑
        // 例如解析JSON格式的验证错误
        vec![json!({
            "field": "unknown",
            "message": msg,
            "code": "VALIDATION_FAILED"
        })]
    }
}

/// 数据库异常处理器 - 专门处理数据库相关错误
/// 🔥 用户只需要添加这两个注解，框架自动完成注册！
#[derive(ExceptionHandler, Component)]
#[bean("databaseExceptionHandler")]
pub struct DatabaseExceptionHandler {
    #[value("app.debug", default = false)]
    debug_mode: bool,
}

#[async_trait::async_trait]
impl GlobalExceptionHandler for DatabaseExceptionHandler {
    fn name(&self) -> &str {
        "DatabaseExceptionHandler"
    }

    fn priority(&self) -> i32 {
        20 // 中等优先级
    }

    fn can_handle(&self, error: &(dyn std::error::Error + Send + Sync)) -> bool {
        // 可以检查具体的数据库错误类型
        // 例如 sqlx::Error, diesel::Error 等
        error.to_string().contains("database")
            || error.to_string().contains("connection")
            || error.to_string().contains("sql")
    }

    async fn handle_error(
        &self,
        error: &(dyn std::error::Error + Send + Sync),
        request_path: &str,
    ) -> Option<ErrorResponse> {
        let mut response = ErrorResponse::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Database Error".to_string(),
            if self.debug_mode {
                error.to_string()
            } else {
                "Database operation failed".to_string()
            },
            request_path.to_string(),
        );

        if self.debug_mode {
            response = response.with_trace(format!("{:?}", error));
        }

        // 添加数据库错误的详细信息
        response = response.with_details(json!({
            "error_type": "DATABASE_ERROR",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "recoverable": self.is_recoverable_error(error),
        }));

        Some(response)
    }
}

impl DatabaseExceptionHandler {
    fn is_recoverable_error(&self, error: &(dyn std::error::Error + Send + Sync)) -> bool {
        let error_str = error.to_string().to_lowercase();
        // 某些数据库错误是可恢复的（例如连接超时）
        error_str.contains("timeout")
            || error_str.contains("connection refused")
            || error_str.contains("connection reset")
    }
}


#[derive(ExceptionHandler, Component)]
#[bean("otherExceptionHandler")]
pub struct OtherExceptionHandler {
    #[value("app.debug", default = true)]
    debug_mode: bool,

    #[value("app.allow-ip-list", default = "127.0.0.1, localhost")]
    allow_ip_list: Vec<String>,

    #[value("app.allowed-ports", default = "8080, 9000, 3000")]
    allowed_ports: Vec<i32>,
}

#[async_trait::async_trait]
impl GlobalExceptionHandler for OtherExceptionHandler {

    fn name(&self) -> &str {
        "OtherExceptionHandler"
    }

    fn priority(&self) -> i32 {
        100 // 低优先级
    }

    fn can_handle(&self, _error: &(dyn std::error::Error + Send + Sync)) -> bool {
        true
    }

    async fn handle_error(
        &self,
        error: &(dyn std::error::Error + Send + Sync),
        request_path: &str,
    ) -> Option<ErrorResponse> {

        println!("debug_mode: {}", self.debug_mode);
        println!("allow_ip_list: {:?}", self.allow_ip_list);
        println!("allowed_ports: {:?}", self.allowed_ports);

        let mut response = ErrorResponse::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "internal_server_error".to_string(),
            if self.debug_mode {
                error.to_string()
            } else {
                "system error".to_string()
            },
            request_path.to_string(),
        );
        
        if self.debug_mode {
            response = response.with_trace(format!("{:?}", error));
        }

        // 添加错误的详细信息
        response = response.with_details(json!({
            "error_type": "SYSTEM_ERROR",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "recoverable": true,
        }));

        Some(response)
    }

}
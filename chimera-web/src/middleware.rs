//! 中间件模块
//!
//! 提供常用的 Web 中间件

use axum::{
    extract::Request,
    http::Uri,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use futures_util::FutureExt;
use std::{sync::Arc, time::Instant};

use crate::exception_handler::GlobalExceptionHandlerRegistry;

/// 请求日志中间件
pub async fn request_logging(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;

    let elapsed = start.elapsed();
    let status = response.status();

    tracing::info!(
        method = %method,
        uri = %uri,
        status = %status.as_u16(),
        elapsed = ?elapsed,
        "Request completed"
    );

    response
}

/// 请求 ID 中间件
pub async fn request_id(mut req: Request, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();

    req.headers_mut()
        .insert("X-Request-ID", request_id.parse().unwrap());

    let mut response = next.run(req).await;

    response
        .headers_mut()
        .insert("X-Request-ID", request_id.parse().unwrap());

    response
}

/// 全局异常处理中间件
pub async fn global_exception_handler(
    uri: Uri,
    Extension(registry): Extension<Arc<GlobalExceptionHandlerRegistry>>,
    req: Request,
    next: Next,
) -> Response {
    let path = uri.path().to_string();

    // 运行下一个处理器并捕获panic
    let response = match std::panic::AssertUnwindSafe(next.run(req))
        .catch_unwind()
        .await
    {
        Ok(response) => response,
        Err(panic) => {
            // 处理panic
            let error_msg = if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic occurred".to_string()
            };

            tracing::error!(path = %path, error = %error_msg, "Handler panicked");

            // 创建一个简单的Error来表示panic
            let panic_error = std::io::Error::new(std::io::ErrorKind::Other, error_msg);

            // 使用异常处理器注册表处理panic
            let error_response = registry.handle_error(&panic_error, &path).await;

            return error_response.into_response();
        }
    };

    // 检查是否是错误响应（状态码4xx或5xx）
    if response.status().is_client_error() || response.status().is_server_error() {
        // 对于错误状态码，可以在这里进行额外的日志记录或处理
        tracing::warn!(
            status = %response.status(),
            path = %path,
            "Request completed with error status"
        );
    }

    response
}

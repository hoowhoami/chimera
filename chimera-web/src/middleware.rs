//! 中间件模块
//!
//! 提供常用的 Web 中间件

use axum::{
    extract::Request,
    http::{StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use chimera_core::ApplicationContext;
use futures_util::FutureExt;
use std::{sync::Arc, time::Instant};

use crate::{
    exception_handler::{ErrorResponse, GlobalExceptionHandlerRegistry},
    interceptor::{InterceptorError, InterceptorRegistry},
};

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
    Extension(_registry): Extension<Arc<GlobalExceptionHandlerRegistry>>,
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

            let error_response = ErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
                "An unexpected error occurred".to_string(),
                path,
            );

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

/// 请求/响应拦截器中间件
pub async fn interceptor_middleware(
    Extension(registry): Extension<Arc<InterceptorRegistry>>,
    Extension(context): Extension<Arc<ApplicationContext>>,
    mut req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    // 1. 执行 pre_handle
    let continue_processing = match registry.pre_handle(&mut req, &context).await {
        Ok(result) => result,
        Err(e) => {
            tracing::warn!(
                error = %e,
                path = %path,
                "Pre-handle interceptor failed"
            );
            return create_interceptor_error_response(&e, &path);
        }
    };

    if !continue_processing {
        // 请求被拦截器终止
        return create_interceptor_error_response(&InterceptorError::AccessDenied, &path);
    }

    // 2. 执行下一个处理器（控制器方法）
    // 注意：这里我们需要克隆请求的一些部分来避免借用检查问题
    let req_method = req.method().clone();
    let req_uri = req.uri().clone();
    let mut response = next.run(req).await;

    // 创建一个简单的请求代理来传递给post_handle和after_completion
    let dummy_req = axum::http::Request::builder()
        .method(req_method)
        .uri(req_uri)
        .body(axum::body::Body::empty())
        .unwrap();

    let mut error: Option<Box<dyn std::error::Error + Send + Sync>> = None;

    // 3. 执行 post_handle
    if let Err(e) = registry
        .post_handle(&dummy_req, &mut response, &context)
        .await
    {
        tracing::error!(
            error = %e,
            path = %path,
            "Post-handle interceptor failed"
        );
        error = Some(Box::new(e));
        // 可以选择修改响应以反映错误，但这里保持原响应
    }

    // 4. 执行 after_completion（无论是否有错误都要执行）
    registry
        .after_completion(&dummy_req, &response, error.as_deref(), &context)
        .await;

    response
}

fn create_interceptor_error_response(error: &InterceptorError, path: &str) -> Response {
    let error_response = ErrorResponse::new(
        error.status_code(),
        "Interceptor Error".to_string(),
        error.error_message(),
        path.to_string(),
    );

    error_response.into_response()
}

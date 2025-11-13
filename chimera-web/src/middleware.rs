//! 中间件模块
//!
//! 提供常用的 Web 中间件

use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use std::time::Instant;

/// 请求日志中间件
pub async fn request_logging(req: Request<Body>, next: Next) -> Response<Body> {
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
pub async fn request_id(mut req: Request<Body>, next: Next) -> Response<Body> {
    let request_id = uuid::Uuid::new_v4().to_string();

    req.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    let mut response = next.run(req).await;

    response.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    response
}

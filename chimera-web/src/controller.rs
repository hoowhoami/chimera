//! 控制器支持
//!
//! 提供类似 Spring MVC 的控制器功能

use axum::{
    http::{StatusCode, HeaderMap, HeaderName, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// HTTP 响应实体
///
/// 类似 Spring 的 ResponseEntity，允许完全控制 HTTP 响应
/// 包括状态码、响应头和响应体
#[derive(Debug)]
pub struct ResponseEntity<T> {
    status: StatusCode,
    headers: HeaderMap,
    body: Option<T>,
}

impl<T> ResponseEntity<T> {
    /// 创建一个新的响应实体
    pub fn new(status: StatusCode, body: T) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Some(body),
        }
    }

    /// 创建一个 200 OK 响应
    pub fn ok(body: T) -> Self {
        Self::new(StatusCode::OK, body)
    }

    /// 创建一个 201 Created 响应
    pub fn created(body: T) -> Self {
        Self::new(StatusCode::CREATED, body)
    }

    /// 创建一个 204 No Content 响应
    pub fn no_content() -> ResponseEntity<()> {
        ResponseEntity {
            status: StatusCode::NO_CONTENT,
            headers: HeaderMap::new(),
            body: None,
        }
    }

    /// 创建一个 400 Bad Request 响应
    pub fn bad_request(body: T) -> Self {
        Self::new(StatusCode::BAD_REQUEST, body)
    }

    /// 创建一个 404 Not Found 响应
    pub fn not_found(body: T) -> Self {
        Self::new(StatusCode::NOT_FOUND, body)
    }

    /// 创建一个 500 Internal Server Error 响应
    pub fn internal_error(body: T) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, body)
    }

    /// 添加响应头
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// 设置状态码
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
}

impl<T> IntoResponse for ResponseEntity<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let mut response = match self.body {
            Some(body) => (self.status, Json(body)).into_response(),
            None => self.status.into_response(),
        };

        // 添加自定义响应头
        let headers = response.headers_mut();
        for (name, value) in self.headers {
            if let Some(name) = name {
                headers.insert(name, value);
            }
        }

        response
    }
}

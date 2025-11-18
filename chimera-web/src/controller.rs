//! 控制器支持
//!
//! 提供类似 Spring MVC 的控制器功能

use axum::{
    http::{StatusCode, HeaderMap, HeaderName, HeaderValue},
    response::{IntoResponse, Response},
    Router, Json,
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

/// 路由信息
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteInfo {
    /// HTTP 方法
    pub method: &'static str,
    /// 完整路径（包含基础路径）
    pub path: String,
}

/// 控制器注册信息
///
/// 用于自动发现和注册控制器
pub struct ControllerRegistration {
    /// 控制器类型名称
    pub type_name: &'static str,

    /// 基础路径
    pub base_path: &'static str,

    /// 路由注册函数（接受无state的Router，返回无state的Router）
    pub register: fn(Router) -> Router,

    /// 获取路由列表的函数（用于冲突检测）
    pub get_route_list: fn() -> &'static [(&'static str, &'static str)], // (method, path)
}

impl ControllerRegistration {
    /// 获取所有路由信息
    pub fn get_routes(&self) -> Vec<RouteInfo> {
        (self.get_route_list)()
            .iter()
            .map(|(method, path)| {
                let full_path = if self.base_path.is_empty() {
                    path.to_string()
                } else if path.starts_with('/') {
                    format!("{}{}", self.base_path, path)
                } else {
                    format!("{}/{}", self.base_path, path)
                };
                RouteInfo {
                    method,
                    path: full_path,
                }
            })
            .collect()
    }
}

// 使用 inventory 收集所有控制器
chimera_core::inventory::collect!(ControllerRegistration);

/// 获取所有注册的控制器
pub fn get_all_controllers() -> impl Iterator<Item = &'static ControllerRegistration> {
    chimera_core::inventory::iter::<ControllerRegistration>()
}

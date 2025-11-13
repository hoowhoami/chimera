//! # Chimera Web
//!
//! Spring Boot 风格的 Rust Web 框架，基于 Axum 构建
//!
//! ## 核心特性
//!
//! - **自动装配** - Web 服务器自动配置和启动
//! - **依赖注入** - 无缝集成 Chimera Core 的依赖注入系统
//! - **注解驱动** - 使用宏实现 @Controller、@RequestMapping 等
//! - **类型安全** - 基于 Axum 的类型安全提取器
//! - **中间件支持** - 集成 Tower 中间件生态系统

pub mod server;
pub mod extractors;
pub mod controller;
pub mod middleware;
pub mod plugin;

pub mod prelude {
    //! 预导入模块

    pub use crate::server::*;
    pub use crate::extractors::*;
    pub use crate::controller::*;
    pub use crate::middleware::*;
    pub use crate::plugin::*;

    pub use axum;
    pub use axum::routing::{get, post, put, delete, patch};
    pub use axum::Router;
    pub use axum::extract::{State, Path, Query, Json};
    pub use axum::response::{IntoResponse, Response};
    pub use axum::http::StatusCode;
}

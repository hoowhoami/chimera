//! Web 服务器模块
//!
//! 基于 Axum 的 Web 服务器实现

use axum::Router;
use chimera_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;

/// Web 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProperties {
    /// 服务器监听地址
    pub host: String,

    /// 服务器监听端口
    pub port: u16,

    /// 工作线程数（0 表示使用 CPU 核心数）
    pub workers: usize,

    /// 请求超时时间（秒）
    pub request_timeout: u64,

    /// 是否启用 CORS
    pub enable_cors: bool,

    /// 是否启用请求日志
    pub enable_request_logging: bool,
}

impl Default for ServerProperties {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 0,
            request_timeout: 30,
            enable_cors: false,
            enable_request_logging: true,
        }
    }
}

impl ServerProperties {
    /// 从 Environment 加载配置
    pub fn from_environment(env: &Environment) -> Self {
        Self {
            host: env
                .get_string("server.host")
                .unwrap_or_else(|| "0.0.0.0".to_string()),
            port: env.get_i64("server.port").unwrap_or(8080) as u16,
            workers: env.get_i64("server.workers").unwrap_or(0) as usize,
            request_timeout: env.get_i64("server.request-timeout").unwrap_or(30) as u64,
            enable_cors: env.get_bool("server.enable-cors").unwrap_or(false),
            enable_request_logging: env
                .get_bool("server.enable-request-logging")
                .unwrap_or(true),
        }
    }

    /// 获取服务器地址
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

///  Chimera Web 服务器
pub struct ChimeraWebServer {
    /// 服务器配置
    config: Arc<ServerProperties>,

    /// 应用上下文
    context: Arc<ApplicationContext>,

    /// 路由（无状态）
    router: Option<Router>,
}

impl ChimeraWebServer {
    /// 创建新的 Web 服务器
    pub async fn new(context: Arc<ApplicationContext>) -> ApplicationResult<Self> {
        // 从容器获取配置
        let config = context
            .get_bean_by_type::<ServerProperties>()
            .await
            .map_err(|e| {
                ApplicationError::Other(format!(
                    "Failed to get ServerProperties bean: {}.",
                    e
                ))
            })?;

        Ok(Self {
            config,
            context,
            router: None,
        })
    }

    /// 设置路由
    pub fn with_router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    /// 启动服务器
    pub async fn run(self) -> ApplicationResult<()> {
        let addr = self.config.address();

        // 获取路由，如果没有则创建空路由
        let app = self.router
            .unwrap_or_else(|| Router::new())
            .into_make_service();

        tracing::info!("🚀 Starting Chimera Web Server on {}", addr);

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| ApplicationError::Other(format!("Failed to bind to {}: {}", addr, e)))?;

        tracing::info!("✅ Server listening on http://{}", addr);

        axum::serve(listener, app)
            .await
            .map_err(|e| ApplicationError::Other(format!("Server error: {}", e)))?;

        Ok(())
    }
}

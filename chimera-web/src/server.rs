//! Web 服务器模块
//!
//! 基于 Axum 的 Web 服务器实现

use axum::{middleware, Router, Extension};
use chimera_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::{
    exception_handler::{build_exception_handler_registry, GlobalExceptionHandlerRegistry},
    middleware::{global_exception_handler, request_id, request_logging},
    controller::get_all_controllers,
};

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

    /// 是否启用全局异常处理
    pub enable_global_exception_handling: bool,
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
            enable_global_exception_handling: true,
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
            enable_global_exception_handling: env
                .get_bool("server.enable-global-exception-handling")
                .unwrap_or(true),
        }
    }

    /// 获取服务器地址
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Chimera Web 服务器
pub struct ChimeraWebServer {
    /// 服务器配置
    config: Arc<ServerProperties>,

    /// 应用上下文
    context: Arc<ApplicationContext>,

    /// 路由（无状态）
    router: Option<Router>,

    /// 异常处理器注册表
    exception_registry: Option<Arc<GlobalExceptionHandlerRegistry>>,
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
            exception_registry: None,
        })
    }

    /// 设置路由
    pub fn with_router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    /// 初始化异常处理器和拦截器
    pub async fn initialize_middleware(mut self) -> ApplicationResult<Self> {

        // 初始化异常处理器注册表
        if self.config.enable_global_exception_handling {
            let exception_registry = build_exception_handler_registry(&self.context).await?;
            tracing::info!("Global exception handling enabled");
            self.exception_registry = Some(Arc::new(exception_registry));
        }

        Ok(self)
    }

    /// 自动注册所有控制器路由
    pub fn auto_register_controllers(mut self) -> Self {
        let mut router = self.router.unwrap_or_else(|| Router::new());

        // 自动注册所有控制器路由
        for controller in get_all_controllers() {
            tracing::info!("Registering controller: {} at {}",
                controller.type_name,
                controller.base_path
            );
            router = (controller.register)(router);
        }

        self.router = Some(router);
        self
    }

    /// 应用中间件
    pub fn with_middleware(mut self) -> Self {
        let mut router = self.router.unwrap_or_else(|| Router::new());

        // 应用中间件（注意顺序：后添加的先执行）
        if self.config.enable_global_exception_handling && self.exception_registry.is_some() {
            router = router.layer(middleware::from_fn(global_exception_handler));
            tracing::debug!("Applied global exception handling middleware");
        }

        if self.config.enable_request_logging {
            router = router.layer(middleware::from_fn(request_logging));
            tracing::debug!("Applied request logging middleware");
        }

        // 请求ID中间件通常是最外层的
        router = router.layer(middleware::from_fn(request_id));
        tracing::debug!("Applied request ID middleware");

        // 添加Extension层（这些需要在中间件之后添加，这样它们会在外层先执行，将数据注入request）
        // 添加共享状态
        router = router.layer(Extension(self.context.clone()));

        // 添加异常处理器和拦截器到Extension（如果启用的话）
        if let Some(exception_registry) = &self.exception_registry {
            router = router.layer(Extension(exception_registry.clone()));
        }

        self.router = Some(router);
        self
    }

    /// 便捷方法：完整的自动配置
    pub async fn auto_configure(self) -> ApplicationResult<Self> {
        Ok(self.initialize_middleware()
            .await?
            .auto_register_controllers()
            .with_middleware())
    }

    /// 启动服务器
    pub async fn run(self) -> ApplicationResult<()> {
        let addr = self.config.address();

        // 获取路由，如果没有则创建空路由
        let app = self.router
            .unwrap_or_else(|| Router::new())
            .into_make_service();

        tracing::info!("Starting Chimera Web Server");
        tracing::info!("Server address: http://{}", addr);
        tracing::info!("Configuration:");
        tracing::info!("  - Global exception handling: {}",
            if self.config.enable_global_exception_handling { "enabled" } else { "disabled" });
        tracing::info!("  - Request logging: {}",
            if self.config.enable_request_logging { "enabled" } else { "disabled" });

        if let Some(_exception_registry) = &self.exception_registry {
            // 这里可以添加日志显示注册了多少个异常处理器，但需要在registry中添加方法
        }

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| ApplicationError::Other(format!("Failed to bind to {}: {}", addr, e)))?;

        tracing::info!("Server ready! Listening on http://{}", addr);

        axum::serve(listener, app)
            .await
            .map_err(|e| ApplicationError::Other(format!("Server error: {}", e)))?;

        Ok(())
    }
}

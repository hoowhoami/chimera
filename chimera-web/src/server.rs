//! Web 服务器模块
//!
//! 基于 Axum 的 Web 服务器实现

use axum::{middleware, Router, Extension};
use chimera_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::{
    constants::*,
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
                .get_string(SERVER_HOST)
                .unwrap_or_else(|| "0.0.0.0".to_string()),
            port: env.get_i64(SERVER_PORT).unwrap_or(8080) as u16,
            workers: env.get_i64(SERVER_WORKERS).unwrap_or(0) as usize,
            request_timeout: env.get_i64(SERVER_REQUEST_TIMEOUT).unwrap_or(30) as u64,
            enable_cors: env.get_bool(SERVER_ENABLE_CORS).unwrap_or(false),
            enable_request_logging: env
                .get_bool(SERVER_ENABLE_REQUEST_LOGGING)
                .unwrap_or(true),
            enable_global_exception_handling: env
                .get_bool(SERVER_ENABLE_GLOBAL_EXCEPTION_HANDLING)
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
    pub async fn new(context: Arc<ApplicationContext>) -> Result<Self> {
        // 从容器获取配置
        let config = context
            .get_bean_by_type::<ServerProperties>()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get ServerProperties bean: {}.",
                    e
                )
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
    pub async fn initialize_middleware(mut self) -> Result<Self> {
        // 初始化异常处理器注册表
        if self.config.enable_global_exception_handling {
            let exception_registry = build_exception_handler_registry(&self.context).await?;
            tracing::info!("Global exception handling enabled");
            self.exception_registry = Some(Arc::new(exception_registry));
        }

        Ok(self)
    }

    /// 自动注册所有控制器路由
    pub fn auto_register_controllers(mut self) -> Result<Self> {
        use std::collections::HashMap;

        let mut router = self.router.unwrap_or_else(|| Router::new());

        // 收集所有路由并检测冲突
        let mut route_map: HashMap<(String, String), Vec<String>> = HashMap::new(); // (method, path) -> [controller_names]

        let controllers: Vec<_> = get_all_controllers().collect();

        // 第一遍：收集所有路由并检测冲突
        for controller in &controllers {
            let routes = controller.get_routes();

            for route in routes {
                let key = (route.method.to_string(), route.path.clone());
                route_map.entry(key)
                    .or_insert_with(Vec::new)
                    .push(controller.type_name.to_string());
            }
        }

        // 检测冲突
        let mut conflicts = Vec::new();
        for ((method, path), controller_names) in &route_map {
            if controller_names.len() > 1 {
                conflicts.push((method.clone(), path.clone(), controller_names.clone()));
            }
        }

        if !conflicts.is_empty() {
            // 构建详细的错误信息
            let mut error_msg = String::from("Route conflicts detected:\n\n");
            for (method, path, controllers) in conflicts {
                error_msg.push_str(&format!(
                    "  ❌ Route conflict: {} {}\n",
                    method, path
                ));
                error_msg.push_str("     Defined in:\n");
                for controller in controllers {
                    error_msg.push_str(&format!("       - {}\n", controller));
                }
                error_msg.push('\n');
            }
            error_msg.push_str("Please resolve these route conflicts before starting the server.");

            tracing::error!("{}", error_msg);
            return Err(anyhow::anyhow!("{}", error_msg));
        }

        // 第二遍：注册所有控制器路由
        for controller in controllers {
            tracing::info!("Registering controller: {} at {}",
                controller.type_name,
                controller.base_path
            );
            router = (controller.register)(router);
        }

        self.router = Some(router);
        Ok(self)
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
    pub async fn auto_configure(self) -> Result<Self> {
        Ok(self.initialize_middleware()
            .await?
            .auto_register_controllers()?
            .with_middleware())
    }

    /// 启动服务器
    pub async fn run(self) -> Result<()> {
        let addr = self.config.address();

        // 获取路由，如果没有则创建空路由
        let app = self.router
            .unwrap_or_else(|| Router::new())
            .into_make_service();

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

        tracing::info!(
            "Web Server (Axum) started on port(s): {} (http)",
            self.config.port
        );

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}

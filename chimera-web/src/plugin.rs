//! Chimera Web 插件
//!
//! 提供 Web 自动装配支持

use chimera_core::prelude::*;
use crate::server::ServerProperties;
use crate::server::ChimeraWebServer;
use axum::Router;
use std::sync::Arc;

/// Web 应用插件
///
/// 自动装配 Web 服务器
pub struct WebPlugin;

impl Default for WebPlugin {
    fn default() -> Self {
        Self
    }
}

#[chimera_core::async_trait::async_trait]
impl ApplicationPlugin for WebPlugin {
    fn name(&self) -> &str {
        "chimera-web"
    }

    fn priority(&self) -> i32 {
        90 // Web 插件优先级较低，在其他插件之后配置
    }

    /// 配置阶段 - 注册 ServerProperties
    fn configure(&self, context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        let env = Arc::clone(context.environment());

        // 注册 ServerProperties Bean
        context
            .register_singleton("serverProperties", move || {
                let env = Arc::clone(&env);
                async move { Ok(ServerProperties::from_environment(&env)) }
            })
            .map_err(|e| ApplicationError::Container(e))?;

        tracing::info!("✅ ServerProperties configured");
        Ok(())
    }

    /// 启动阶段 - 启动 Web 服务器
    async fn on_startup(&self, context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        // 创建基础路由器（无状态）
        let mut router = Router::new();

        // 自动注册所有控制器
        let controller_count = crate::controller::get_all_controllers().count();

        if controller_count > 0 {
            tracing::info!("🎯 Registering {} controllers...", controller_count);

            for registration in crate::controller::get_all_controllers() {
                tracing::debug!(
                    "  Registering controller '{}' at path '{}'",
                    registration.type_name,
                    registration.base_path
                );

                // 调用控制器的注册函数
                router = (registration.register)(router);
            }

            tracing::info!("✅ All controllers registered successfully");
        } else {
            tracing::info!("ℹ️  No controllers found (this is ok for non-web apps)");
        }

        // 添加 ApplicationContext 作为 Extension，这样所有提取器都能访问它
        let router = router.layer(axum::Extension(Arc::clone(context)));

        // 创建并启动服务器（在后台运行）
        let context_clone = Arc::clone(context);
        tokio::spawn(async move {
            match ChimeraWebServer::new(context_clone).await {
                Ok(server) => {
                    if let Err(e) = server.with_router(router).run().await {
                        tracing::error!("Web server error: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create web server: {}", e);
                }
            }
        });

        Ok(())
    }

    /// 关闭阶段
    async fn on_shutdown(&self, _context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        tracing::info!("Web server shutting down");
        Ok(())
    }
}

// 自动提交插件到全局注册表
chimera_core::inventory::submit! {
    chimera_core::PluginSubmission {
        create: || Box::new(WebPlugin::default())
    }
}

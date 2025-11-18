//! AOP Plugin - 通过插件机制集成 AOP 到框架
//!
//! 提供 ApplicationPlugin 实现，自动注册 AOP BeanPostProcessor

use chimera_core::prelude::*;
use async_trait::async_trait;
use std::sync::Arc;

/// AOP 应用插件
///
/// 通过实现 ApplicationPlugin，自动将 AOP 功能集成到应用中
///
/// ## 功能
///
/// 1. 自动加载所有通过 inventory 注册的切面
/// 2. 注册 AopBeanPostProcessor 到容器
/// 3. BeanPostProcessor 会在 Bean 创建后自动为符合条件的 Bean 应用 AOP
///
/// ## 使用方式
///
/// ### 方式一：自动加载（推荐）
///
/// AopPlugin 会通过 inventory 机制自动注册，无需手动添加：
///
/// ```ignore
/// use chimera_core::prelude::*;
///
/// #[tokio::main]
/// async fn main() -> ApplicationResult<()> {
///     ChimeraApplication::new()
///         .run()
///         .await
/// }
/// ```
///
/// ### 方式二：显式添加
///
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_aop::AopPlugin;
///
/// #[tokio::main]
/// async fn main() -> ApplicationResult<()> {
///     ChimeraApplication::new()
///         .add_plugin(Arc::new(AopPlugin::new()))
///         .run()
///         .await
/// }
/// ```
pub struct AopPlugin {
    /// 插件名称
    name: String,
    /// 是否启用
    enabled: bool,
}

impl AopPlugin {
    /// 创建新的 AOP 插件
    pub fn new() -> Self {
        Self {
            name: "AopPlugin".to_string(),
            enabled: true,
        }
    }

    /// 创建禁用的 AOP 插件
    pub fn disabled() -> Self {
        Self {
            name: "AopPlugin".to_string(),
            enabled: false,
        }
    }

    /// 设置插件名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl Default for AopPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApplicationPlugin for AopPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn configure(&self, context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        if !self.enabled {
            tracing::info!("AOP Plugin is disabled, skipping initialization");
            return Ok(());
        }

        tracing::info!("🔷 [AopPlugin] Initializing AOP support...");

        // 获取全局 AOP 注册表（会自动加载所有切面）
        let registry = crate::get_global_registry();
        tracing::info!("🔷 [AopPlugin] Loaded {} aspect(s) from global registry", registry.len());

        // 注册 AopBeanPostProcessor
        // 注意：configure 是同步方法，但 add_bean_post_processor 是异步方法
        // 我们需要在运行时阻塞调用异步方法
        let aop_processor = Arc::new(crate::AopBeanPostProcessor::new());

        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                context.add_bean_post_processor(aop_processor).await;
            })
        });

        tracing::info!("🔷 [AopPlugin] AOP BeanPostProcessor registered successfully");
        tracing::info!("🔷 [AopPlugin] AOP support initialized - Service beans will be automatically wrapped with AOP");

        Ok(())
    }

    async fn on_startup(&self, _context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        if self.enabled {
            tracing::info!("🔷 [AopPlugin] AOP is active and monitoring Service beans");
        }
        Ok(())
    }

    async fn on_shutdown(&self, _context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        if self.enabled {
            tracing::info!("🔷 [AopPlugin] Shutting down AOP support");
        }
        Ok(())
    }
}

// 自动注册 AOP 插件到 inventory
chimera_core::inventory::submit! {
    chimera_core::PluginSubmission {
        create: || Box::new(AopPlugin::new()) as Box<dyn ApplicationPlugin>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = AopPlugin::new();
        assert_eq!(plugin.name(), "AopPlugin");
        assert!(plugin.enabled);
    }

    #[test]
    fn test_disabled_plugin() {
        let plugin = AopPlugin::disabled();
        assert!(!plugin.enabled);
    }

    #[test]
    fn test_custom_name() {
        let plugin = AopPlugin::new().with_name("CustomAopPlugin");
        assert_eq!(plugin.name(), "CustomAopPlugin");
    }
}

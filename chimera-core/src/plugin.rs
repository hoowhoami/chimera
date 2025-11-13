//! 插件机制
//!
//! 提供应用插件的定义和管理，支持模块化扩展

use crate::prelude::*;
use async_trait::async_trait;
use std::sync::Arc;

/// 应用插件 trait
///
/// 实现此 trait 以创建可插拔的应用模块
#[async_trait]
pub trait ApplicationPlugin: Send + Sync {
    /// 插件名称
    fn name(&self) -> &str;

    /// 插件优先级（数字越小优先级越高）
    fn priority(&self) -> i32 {
        100
    }

    /// 配置阶段 - 在组件扫描之前执行
    ///
    /// 用于注册额外的 Bean、配置源等
    fn configure(&self, _context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        Ok(())
    }

    /// 启动阶段 - 在应用完全初始化后执行
    ///
    /// 用于启动额外的服务，如 Web 服务器
    async fn on_startup(&self, _context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        Ok(())
    }

    /// 关闭阶段 - 在应用关闭时执行
    ///
    /// 用于清理资源
    async fn on_shutdown(&self, _context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        Ok(())
    }
}

/// 插件注册表
pub struct PluginRegistry {
    plugins: Vec<Box<dyn ApplicationPlugin>>,
}

impl PluginRegistry {
    /// 创建新的插件注册表
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// 注册插件
    pub fn register(&mut self, plugin: Box<dyn ApplicationPlugin>) {
        tracing::debug!("Registering plugin: {}", plugin.name());
        self.plugins.push(plugin);
    }

    /// 按优先级排序插件
    pub fn sort_by_priority(&mut self) {
        self.plugins.sort_by_key(|p| p.priority());
    }

    /// 获取所有插件
    pub fn plugins(&self) -> &[Box<dyn ApplicationPlugin>] {
        &self.plugins
    }

    /// 执行配置阶段
    pub fn configure_all(&self, context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        for plugin in &self.plugins {
            tracing::info!("⚙️  Configuring plugin: {}", plugin.name());
            plugin.configure(context)?;
        }
        Ok(())
    }

    /// 执行启动阶段
    pub async fn startup_all(&self, context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        for plugin in &self.plugins {
            tracing::info!("🚀 Starting plugin: {}", plugin.name());
            plugin.on_startup(context).await?;
        }
        Ok(())
    }

    /// 执行关闭阶段
    pub async fn shutdown_all(&self, context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        // 逆序关闭
        for plugin in self.plugins.iter().rev() {
            tracing::info!("🛑 Shutting down plugin: {}", plugin.name());
            if let Err(e) = plugin.on_shutdown(context).await {
                tracing::error!("Failed to shutdown plugin {}: {}", plugin.name(), e);
            }
        }
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 用于全局收集插件的宏
#[macro_export]
macro_rules! submit_plugin {
    ($plugin_type:ty) => {
        $crate::inventory::submit! {
            $crate::PluginSubmission {
                create: || Box::new(<$plugin_type>::default())
            }
        }
    };
}

/// 插件提交结构
pub struct PluginSubmission {
    pub create: fn() -> Box<dyn ApplicationPlugin>,
}

inventory::collect!(PluginSubmission);

/// 从全局注册表加载所有插件
pub fn load_plugins() -> PluginRegistry {
    let mut registry = PluginRegistry::new();

    for submission in inventory::iter::<PluginSubmission> {
        registry.register((submission.create)());
    }

    registry.sort_by_priority();
    registry
}

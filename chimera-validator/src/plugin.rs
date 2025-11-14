use chimera_core::prelude::*;
use chimera_core::plugin::ApplicationPlugin;
use std::sync::Arc;

/// 验证器插件
pub struct ValidatorPlugin;

impl ValidatorPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[chimera_core::async_trait::async_trait]
impl ApplicationPlugin for ValidatorPlugin {
    fn name(&self) -> &str {
        "chimera-validator"
    }

    fn configure(&self, _context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        tracing::info!("Validator plugin configured");
        Ok(())
    }

    async fn on_startup(&self, _context: &Arc<ApplicationContext>) -> ApplicationResult<()> {
        tracing::info!("Validator plugin started");
        Ok(())
    }
}

impl Default for ValidatorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

// 注册插件到全局注册表
chimera_core::submit_plugin!(ValidatorPlugin);

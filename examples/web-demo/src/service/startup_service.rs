use chimera_core::prelude::*;
use chimera_core_macros::{Component, SmartInitializingSingleton};
use std::sync::Arc;

/// 启动服务 - 演示 SmartInitializingSingleton 接口
///
/// 在所有单例 Bean 初始化完成后执行启动逻辑
#[derive(SmartInitializingSingleton, Component, Clone)]
pub struct StartupService {
    #[autowired]
    app_context: Arc<ApplicationContext>,
}

impl SmartInitializingSingleton for StartupService {
    fn after_singletons_instantiated(&self) -> ContainerResult<()> {
        tracing::info!("🚀 [SmartInitializingSingleton] All singletons initialized!");
        tracing::info!("   - Application context: {}", self.app_context.get_app_name().unwrap_or_else(|| "unnamed".to_string()));
        tracing::info!("   - Starting background tasks...");
        tracing::info!("   - Application is ready to serve requests!");
        Ok(())
    }
}


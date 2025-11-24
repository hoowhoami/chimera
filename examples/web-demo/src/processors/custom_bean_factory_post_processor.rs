//! 自定义 BeanFactoryPostProcessor - 在 Bean 实例化之前修改 Bean 定义
//!
//! 演示如何使用 #[derive(BeanFactoryPostProcessor, Component)] 宏自动注册

use chimera_core::prelude::*;
use chimera_core_macros::{BeanFactoryPostProcessor, Component};
use std::sync::Arc;

/// 自定义 BeanFactoryPostProcessor
///
/// 在所有 Bean 实例化之前执行，可以修改 Bean 定义
/// 
/// 使用场景：
/// - 修改 Bean 的作用域
/// - 添加或修改 Bean 的属性
/// - 动态注册 Bean 定义
/// - 配置占位符解析
#[derive(BeanFactoryPostProcessor, Component)]
pub struct CustomBeanFactoryPostProcessor {
    #[autowired]
    environment: Arc<Environment>,
}

impl BeanFactoryPostProcessor for CustomBeanFactoryPostProcessor {
    fn post_process_bean_factory(&self, context: &ApplicationContext) -> ContainerResult<()> {
        tracing::info!("🔧 [BeanFactoryPostProcessor] Processing bean factory...");

        // 获取 BeanFactory
        let bean_factory = context.get_bean_factory();

        // 获取所有已注册的 Bean 定义
        use chimera_core::bean_factory::ListableBeanFactory;
        let bean_names = bean_factory.as_ref().get_bean_names();
        tracing::info!("   - Total bean definitions: {}", bean_names.len());

        // 获取当前激活的 profiles
        let profiles = self.environment.get_active_profiles();
        tracing::info!("   - Active profiles: {:?}", profiles);

        // 示例：可以在这里修改 Bean 定义
        // 例如：根据环境变量动态修改某些 Bean 的配置
        if profiles.contains(&"dev".to_string()) {
            tracing::info!("   - Development mode detected, applying dev-specific bean configurations");
        }

        // 示例：可以在这里动态注册新的 Bean 定义
        // bean_factory.register_bean_definition(...);

        tracing::info!("🔧 [BeanFactoryPostProcessor] Bean factory processing completed");

        Ok(())
    }
    
    fn order(&self) -> i32 {
        100  // 优先级：数字越小优先级越高
    }
}


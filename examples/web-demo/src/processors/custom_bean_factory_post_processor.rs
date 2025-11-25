//! 自定义 BeanFactoryPostProcessor - 在 Bean 实例化之前修改 Bean 定义
//!
//! 演示如何使用 #[derive(BeanFactoryPostProcessor, Component)] 宏自动注册

use chimera_core::prelude::*;
use chimera_core::bean::FunctionFactory;
use chimera_core_macros::{BeanFactoryPostProcessor, Component};
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

/// 动态配置 Bean - 用于演示动态注册
#[derive(Debug, Clone)]
pub struct DynamicConfig {
    pub profile: String,
    pub modified_at: String,
}

/// 计数器服务 - 用于验证 Prototype 作用域
#[derive(Debug)]
pub struct CounterService {
    counter: AtomicU32,
}

impl CounterService {
    pub fn new() -> Self {
        Self { counter: AtomicU32::new(0) }
    }

    pub fn increment(&self) -> u32 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

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
    fn post_process_bean_factory(&self, context: &ApplicationContext) -> Result<()> {
        tracing::info!("🔧 [BeanFactoryPostProcessor] Processing bean factory...");

        // 获取 BeanFactory
        let bean_factory = context.get_bean_factory();

        // 获取所有已注册的 Bean 定义
        use chimera_core::bean_factory::{ListableBeanFactory, ConfigurableBeanFactory};
        let bean_names = bean_factory.as_ref().get_bean_names();
        tracing::info!("   - Total bean definitions: {}", bean_names.len());

        // 获取当前激活的 profiles
        let profiles = self.environment.get_active_profiles();
        tracing::info!("   - Active profiles: {:?}", profiles);

        // ============ 示例 1：修改现有 Bean 的作用域 ============
        // 在开发模式下，将 userService 改为 Prototype 作用域（每次获取都创建新实例）
        if profiles.contains(&"dev".to_string()) {
            tracing::info!("   - Development mode detected, applying dev-specific bean configurations");

            // 修改 userService 的延迟加载属性
            if bean_factory.contains_bean_definition("userService") {
                bean_factory.modify_bean_definition("userService", |def| {
                    tracing::info!("     ✏️  Modifying 'userService' bean definition:");
                    tracing::info!("        - Original lazy: {}", def.lazy);
                    def.lazy = true;  // 设置为延迟加载
                    tracing::info!("        - Modified lazy: {}", def.lazy);
                })?;
            }
        }

        // ============ 示例 2：动态注册新的 Bean ============
        // 根据当前 profile 动态创建配置 Bean
        tracing::info!("   - Registering dynamic beans...");

        // 注册一个动态配置 Bean
        let profile = profiles.first().cloned().unwrap_or_else(|| "default".to_string());
        let profile_clone = profile.clone();
        let config_factory = FunctionFactory::new(move || {
            Ok(DynamicConfig {
                profile: profile_clone.clone(),
                modified_at: chrono::Utc::now().to_rfc3339(),
            })
        });

        let config_definition = BeanDefinition::new("dynamicConfig", config_factory)
            .with_scope(Scope::Singleton);

        bean_factory.register_bean_definition("dynamicConfig".to_string(), config_definition)?;
        tracing::info!("     ✅ Registered bean: 'dynamicConfig' (Singleton)");

        // 注册一个 Prototype 作用域的计数器服务
        let counter_factory = FunctionFactory::new(|| {
            Ok(CounterService::new())
        });

        let counter_definition = BeanDefinition::new("counterService", counter_factory)
            .with_scope(Scope::Prototype);  // 每次获取都创建新实例

        bean_factory.register_bean_definition("counterService".to_string(), counter_definition)?;
        tracing::info!("     ✅ Registered bean: 'counterService' (Prototype)");

        tracing::info!("🔧 [BeanFactoryPostProcessor] Bean factory processing completed");

        Ok(())
    }

    fn order(&self) -> i32 {
        100  // 优先级：数字越小优先级越高
    }
}


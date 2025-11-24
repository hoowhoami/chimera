//! 测试 BeanFactoryPostProcessor 效果的控制器

use chimera_core::prelude::*;
use chimera_core_macros::{component, Component};
use chimera_web::prelude::*;
use chimera_web_macros::{controller, get_mapping};
use serde_json::json;
use std::sync::Arc;

use crate::processors::custom_bean_factory_post_processor::{DynamicConfig, CounterService};

/// 测试 Bean 控制器
///
/// 用于验证 BeanFactoryPostProcessor 的修改是否生效
#[controller("/test")]
#[derive(Component, Clone)]
pub struct TestBeanController {
    #[autowired]
    app_context: Arc<ApplicationContext>,
}

#[component]
#[controller]
impl TestBeanController {
    /// 测试动态注册的 Bean
    #[get_mapping("/dynamic-config")]
    async fn test_dynamic_config(&self) -> impl IntoResponse {
        use chimera_core::bean_factory::BeanFactoryExt;
        let bean_factory = self.app_context.get_bean_factory();

        // 获取动态注册的配置 Bean
        match bean_factory.get_bean_by_type::<DynamicConfig>() {
            Ok(config) => {
                tracing::info!("✅ Successfully retrieved dynamicConfig bean");
                ResponseEntity::ok(json!({
                    "success": true,
                    "message": "DynamicConfig bean was successfully registered by BeanFactoryPostProcessor",
                    "config": {
                        "profile": config.profile,
                        "modified_at": config.modified_at,
                    }
                }))
            }
            Err(e) => {
                tracing::error!("❌ Failed to retrieve dynamicConfig bean: {}", e);
                ResponseEntity::ok(json!({
                    "success": false,
                    "error": format!("Failed to get dynamicConfig: {}", e)
                }))
            }
        }
    }

    /// 测试 Prototype 作用域的 Bean
    #[get_mapping("/counter")]
    async fn test_counter_service(&self) -> impl IntoResponse {
        use chimera_core::bean_factory::BeanFactory;
        let bean_factory = self.app_context.get_bean_factory();

        // 获取三次 counterService，验证是否是不同的实例
        let counter1 = bean_factory.get_bean("counterService").unwrap();
        let counter2 = bean_factory.get_bean("counterService").unwrap();
        let counter3 = bean_factory.get_bean("counterService").unwrap();

        // 转换为具体类型
        let c1 = counter1.downcast_ref::<CounterService>().unwrap();
        let c2 = counter2.downcast_ref::<CounterService>().unwrap();
        let c3 = counter3.downcast_ref::<CounterService>().unwrap();

        // 每个实例调用 increment
        let val1 = c1.increment();
        let val2 = c2.increment();
        let val3 = c3.increment();

        // 如果是 Prototype 作用域，每个实例的计数器都应该从 0 开始
        // 所以 val1, val2, val3 都应该是 0
        let is_prototype = val1 == 0 && val2 == 0 && val3 == 0;

        tracing::info!(
            "Counter test: val1={}, val2={}, val3={}, is_prototype={}",
            val1, val2, val3, is_prototype
        );

        ResponseEntity::ok(json!({
            "success": true,
            "message": if is_prototype {
                "CounterService is correctly configured as Prototype scope - each get_bean() returns a new instance"
            } else {
                "CounterService appears to be Singleton - same instance returned"
            },
            "scope": if is_prototype { "Prototype" } else { "Singleton" },
            "counter_values": [val1, val2, val3],
            "proof": "If all values are 0, it means each instance started with its own counter (Prototype). If values are [0,1,2], it's the same instance (Singleton)."
        }))
    }

    /// 测试所有注册的 Bean
    #[get_mapping("/beans")]
    async fn list_beans(&self) -> impl IntoResponse {
        use chimera_core::bean_factory::ListableBeanFactory;
        let bean_factory = self.app_context.get_bean_factory();

        let bean_names = bean_factory.as_ref().get_bean_names();

        ResponseEntity::ok(json!({
            "success": true,
            "total": bean_names.len(),
            "beans": bean_names,
            "message": "Check if 'dynamicConfig' and 'counterService' are in the list (registered by BeanFactoryPostProcessor)"
        }))
    }
}

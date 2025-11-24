//! 生命周期测试 BeanPostProcessor
//!
//! 用于验证 Bean 生命周期回调的正确顺序

use chimera_core::prelude::*;
use chimera_core_macros::{BeanPostProcessor, Component};
use std::any::Any;
use std::sync::Arc;

/// 生命周期测试 BeanPostProcessor
///
/// 记录所有 Bean 的生命周期回调，用于验证顺序
#[derive(BeanPostProcessor, Component)]
pub struct LifecycleTestProcessor {
    #[autowired]
    app_context: Arc<ApplicationContext>,
}

impl BeanPostProcessor for LifecycleTestProcessor {
    fn name(&self) -> &str {
        "LifecycleTestProcessor"
    }

    fn order(&self) -> i32 {
        100
    }

    fn post_process_before_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        bean_name: &str,
    ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        tracing::info!("🔵 [Lifecycle] postProcessBeforeInitialization: {}", bean_name);
        Ok(bean)
    }

    fn post_process_after_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        bean_name: &str,
    ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        tracing::info!("🟢 [Lifecycle] postProcessAfterInitialization: {}", bean_name);
        Ok(bean)
    }
}


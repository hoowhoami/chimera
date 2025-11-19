//! 计时 BeanPostProcessor - 统计 Bean 创建耗时
//!
//! 演示如何在 BeanPostProcessor 中维护状态

use chimera_core::prelude::*;
use chimera_core_macros::{BeanPostProcessor, Component};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

/// 计时 BeanPostProcessor
///
/// 统计每个 Bean 的创建耗时，帮助性能分析
#[derive(BeanPostProcessor, Component)]
pub struct TimingBeanPostProcessor {
    /// 记录每个 Bean 开始创建的时间
    start_times: Mutex<HashMap<String, Instant>>,
    
    #[autowired]
    app_context: Arc<ApplicationContext>,
}

impl BeanPostProcessor for TimingBeanPostProcessor {
    fn name(&self) -> &str {
        "TimingBeanPostProcessor"
    }

    fn order(&self) -> i32 {
        200
    }

    fn post_process_before_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        bean_name: &str,
    ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        let mut times = self.start_times.lock().unwrap();
        times.insert(bean_name.to_string(), Instant::now());
        Ok(bean)
    }

    fn post_process_after_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        bean_name: &str,
    ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        let mut times = self.start_times.lock().unwrap();
        if let Some(start_time) = times.remove(bean_name) {
            let elapsed = start_time.elapsed();
            if elapsed.as_millis() > 10 {
                tracing::warn!(
                    "⏱️  [BeanPostProcessor] Bean '{}' took {}ms to initialize (slow!)",
                    bean_name,
                    elapsed.as_millis()
                );
            } else {
                tracing::debug!(
                    "⏱️  [BeanPostProcessor] Bean '{}' initialized in {}ms",
                    bean_name,
                    elapsed.as_millis()
                );
            }
        }
        Ok(bean)
    }
}

//! 日志 BeanPostProcessor - 记录所有 Bean 的创建过程
//!
//! 演示如何使用 #[derive(BeanPostProcessor, Component)] 宏自动注册并支持依赖注入

use chimera_core::prelude::*;
use chimera_core_macros::{BeanPostProcessor, Component};
use std::any::Any;
use std::sync::Arc;

/// 日志 BeanPostProcessor
///
/// 在 Bean 初始化前后记录日志，帮助调试和监控
#[derive(BeanPostProcessor, Component)]
pub struct LoggingBeanPostProcessor;

impl BeanPostProcessor for LoggingBeanPostProcessor {
    fn name(&self) -> &str {
        "LoggingBeanPostProcessor"
    }

    fn order(&self) -> i32 {
        100
    }

    fn post_process_before_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        bean_name: &str,
    ) -> Result<Arc<dyn Any + Send + Sync>> {
        tracing::debug!("📦 [BeanPostProcessor] Before initialization: '{}'", bean_name);
        Ok(bean)
    }

    fn post_process_after_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        bean_name: &str,
    ) -> Result<Arc<dyn Any + Send + Sync>> {
        tracing::info!("✅ [BeanPostProcessor] Bean '{}' initialized successfully", bean_name);
        Ok(bean)
    }
}

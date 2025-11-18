//! 日志切面 - 演示切面作为 Component

use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_aop::prelude::*;
use async_trait::async_trait;
use std::sync::Arc;

/// 日志切面 - 记录所有 Service 方法调用
///
/// 这是一个 Component，可以被依赖注入系统管理
#[derive(Component, Clone)]
pub struct LoggingAspect {
    pointcut: PointcutExpression,
}

impl LoggingAspect {
    pub fn new() -> Self {
        Self {
            pointcut: PointcutExpression::execution("* *Service.*(..)"),
        }
    }
}

#[async_trait]
impl Aspect for LoggingAspect {
    fn name(&self) -> &str {
        "LoggingAspect"
    }

    fn pointcut(&self) -> &PointcutExpression {
        &self.pointcut
    }

    // #[before] 前置通知
    async fn before(&self, jp: &JoinPoint) {
        tracing::info!("🔵 [AOP-Logging] → Entering: {}", jp.signature());
    }

    // #[after] 后置通知
    async fn after(&self, jp: &JoinPoint) {
        let elapsed = jp.timestamp.elapsed();
        tracing::info!("🔵 [AOP-Logging] ← Exiting: {} (took {:?})", jp.signature(), elapsed);
    }
}

// 自动注册到 AOP 系统
chimera_aop::inventory::submit! {
    AspectRegistration::new(
        "LoggingAspect",
        "execution(* *Service.*(..))",
        || Arc::new(LoggingAspect::new()) as Arc<dyn Aspect>
    )
}

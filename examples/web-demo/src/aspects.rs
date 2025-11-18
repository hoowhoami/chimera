//! AOP 切面定义
//!
//! 演示各种切面的使用（使用 inventory 自动注册）

use chimera_aop::prelude::*;
use async_trait::async_trait;
use std::sync::Arc;

/// Service 方法日志切面
pub struct ServiceLoggingAspect {
    pointcut: PointcutExpression,
}

impl ServiceLoggingAspect {
    pub fn new() -> Self {
        Self {
            pointcut: PointcutExpression::execution("* *Service.*(..)")
        }
    }
}

#[async_trait]
impl Aspect for ServiceLoggingAspect {
    fn name(&self) -> &str {
        "ServiceLoggingAspect"
    }

    fn pointcut(&self) -> &PointcutExpression {
        &self.pointcut
    }

    async fn before(&self, jp: &JoinPoint) {
        tracing::info!("🔵 [AOP] → Entering Service method: {}", jp.signature());
    }

    async fn after(&self, jp: &JoinPoint) {
        let elapsed = jp.timestamp.elapsed();
        tracing::info!("🔵 [AOP] ← Exiting Service method: {} (took {:?})",
            jp.signature(), elapsed);
    }
}

// 自动注册切面
chimera_aop::inventory::submit! {
    AspectRegistration::new(
        "ServiceLoggingAspect",
        "execution(* *Service.*(..))",
        || Arc::new(ServiceLoggingAspect::new()) as Arc<dyn Aspect>
    )
}

/// 性能监控切面
pub struct PerformanceMonitorAspect {
    threshold_ms: u128,
    pointcut: PointcutExpression,
}

impl PerformanceMonitorAspect {
    pub fn new() -> Self {
        Self {
            threshold_ms: 50, // 50ms 阈值
            pointcut: PointcutExpression::All,
        }
    }
}

#[async_trait]
impl Aspect for PerformanceMonitorAspect {
    fn name(&self) -> &str {
        "PerformanceMonitorAspect"
    }

    fn pointcut(&self) -> &PointcutExpression {
        &self.pointcut
    }

    async fn after(&self, jp: &JoinPoint) {
        let elapsed = jp.timestamp.elapsed().as_millis();
        if elapsed > self.threshold_ms {
            tracing::warn!("⚠️  [AOP Performance] Slow method detected: {} took {}ms (threshold: {}ms)",
                jp.signature(), elapsed, self.threshold_ms);
        }
    }
}

// 自动注册切面
chimera_aop::inventory::submit! {
    AspectRegistration::new(
        "PerformanceMonitorAspect",
        "*",
        || Arc::new(PerformanceMonitorAspect::new()) as Arc<dyn Aspect>
    )
}

/// 业务事务切面
pub struct TransactionAspect {
    pointcut: PointcutExpression,
}

impl TransactionAspect {
    pub fn new() -> Self {
        Self {
            pointcut: PointcutExpression::execution("* *Service.create*(..)"),
        }
    }
}

#[async_trait]
impl Aspect for TransactionAspect {
    fn name(&self) -> &str {
        "TransactionAspect"
    }

    fn pointcut(&self) -> &PointcutExpression {
        &self.pointcut
    }

    async fn before(&self, jp: &JoinPoint) {
        tracing::info!("📦 [AOP Transaction] Starting transaction for: {}", jp.signature());
    }

    async fn after_returning(&self, jp: &JoinPoint) {
        tracing::info!("✅ [AOP Transaction] Committing transaction for: {}", jp.signature());
    }

    async fn after_throwing(&self, jp: &JoinPoint, error_msg: &str) {
        tracing::error!("❌ [AOP Transaction] Rolling back transaction for: {}: {}",
            jp.signature(), error_msg);
    }
}

// 自动注册切面
chimera_aop::inventory::submit! {
    AspectRegistration::new(
        "TransactionAspect",
        "execution(* *Service.create*(..))",
        || Arc::new(TransactionAspect::new()) as Arc<dyn Aspect>
    )
}

/// 异常处理切面
pub struct ExceptionLoggingAspect {
    pointcut: PointcutExpression,
}

impl ExceptionLoggingAspect {
    pub fn new() -> Self {
        Self {
            pointcut: PointcutExpression::All,
        }
    }
}

#[async_trait]
impl Aspect for ExceptionLoggingAspect {
    fn name(&self) -> &str {
        "ExceptionLoggingAspect"
    }

    fn pointcut(&self) -> &PointcutExpression {
        &self.pointcut
    }

    async fn after_throwing(&self, jp: &JoinPoint, error_msg: &str) {
        tracing::error!("❌ [AOP Exception] Error in {}: {}", jp.signature(), error_msg);
    }
}

// 自动注册切面
chimera_aop::inventory::submit! {
    AspectRegistration::new(
        "ExceptionLoggingAspect",
        "*",
        || Arc::new(ExceptionLoggingAspect::new()) as Arc<dyn Aspect>
    )
}

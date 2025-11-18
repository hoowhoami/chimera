//! 切面（Aspect）定义
//!
//! 切面是横切关注点的模块化

use crate::{JoinPoint, PointcutExpression};
use async_trait::async_trait;
use std::sync::Arc;

/// 切面 Trait
///
/// 实现此 trait 以定义切面逻辑
#[async_trait]
pub trait Aspect: Send + Sync {
    /// 切面名称
    fn name(&self) -> &str;

    /// 切点表达式
    fn pointcut(&self) -> &PointcutExpression;

    /// 前置通知（可选实现）
    async fn before(&self, _join_point: &JoinPoint) {}

    /// 后置通知（可选实现）
    async fn after(&self, _join_point: &JoinPoint) {}

    /// 返回后通知（可选实现）
    /// 当方法正常返回时调用（不包含具体返回值）
    async fn after_returning(&self, _join_point: &JoinPoint) {}

    /// 异常通知（可选实现）
    /// error_msg 是错误消息字符串
    async fn after_throwing(&self, _join_point: &JoinPoint, _error_msg: &str) {}
}

/// 切面注册器
///
/// 用于 inventory 自动收集和注册切面
/// 类似于 Component 的 ComponentRegistry 和 Controller 的ControllerRegistration
pub struct AspectRegistration {
    /// 切面名称
    pub name: &'static str,

    /// 切点表达式
    pub pointcut_expr: &'static str,

    /// 创建切面实例的函数
    pub creator: fn() -> Arc<dyn Aspect>,
}

impl AspectRegistration {
    /// 创建新的切面注册器
    pub const fn new(
        name: &'static str,
        pointcut_expr: &'static str,
        creator: fn() -> Arc<dyn Aspect>,
    ) -> Self {
        Self {
            name,
            pointcut_expr,
            creator,
        }
    }

    /// 创建切面实例
    pub fn create_instance(&self) -> Arc<dyn Aspect> {
        (self.creator)()
    }
}

// 使用 inventory 收集所有切面注册器
inventory::collect!(AspectRegistration);

/// 获取所有注册的切面注册器
pub fn get_all_aspect_registrations() -> impl Iterator<Item = &'static AspectRegistration> {
    inventory::iter::<AspectRegistration>()
}

// ============================================================================
// 预定义的常用切面
// ============================================================================

/// 日志切面 - 记录方法调用
pub struct LoggingAspect {
    log_args: bool,
    log_result: bool,
    pointcut: PointcutExpression,
}

impl LoggingAspect {
    pub fn new(pointcut: PointcutExpression) -> Self {
        Self {
            log_args: false,
            log_result: true,
            pointcut,
        }
    }

    pub fn with_args(mut self) -> Self {
        self.log_args = true;
        self
    }

    pub fn with_result(mut self) -> Self {
        self.log_result = true;
        self
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

    async fn before(&self, join_point: &JoinPoint) {
        tracing::info!("→ Entering: {}", join_point.signature());
    }

    async fn after(&self, join_point: &JoinPoint) {
        let elapsed = join_point.timestamp.elapsed();
        tracing::info!("← Exiting: {} (took {:?})", join_point.signature(), elapsed);
    }
}

/// 性能监控切面
pub struct PerformanceAspect {
    threshold_ms: u128,
    pointcut: PointcutExpression,
}

impl PerformanceAspect {
    pub fn new(threshold_ms: u128, pointcut: PointcutExpression) -> Self {
        Self {
            threshold_ms,
            pointcut,
        }
    }
}

#[async_trait]
impl Aspect for PerformanceAspect {
    fn name(&self) -> &str {
        "PerformanceAspect"
    }

    fn pointcut(&self) -> &PointcutExpression {
        &self.pointcut
    }

    async fn after(&self, join_point: &JoinPoint) {
        let elapsed = join_point.timestamp.elapsed().as_millis();
        if elapsed > self.threshold_ms {
            tracing::warn!(
                "⚠️ Slow method detected: {} took {}ms (threshold: {}ms)",
                join_point.signature(),
                elapsed,
                self.threshold_ms
            );
        }
    }
}

/// 异常处理切面
pub struct ExceptionHandlingAspect {
    pointcut: PointcutExpression,
}

impl ExceptionHandlingAspect {
    pub fn new(pointcut: PointcutExpression) -> Self {
        Self { pointcut }
    }
}

#[async_trait]
impl Aspect for ExceptionHandlingAspect {
    fn name(&self) -> &str {
        "ExceptionHandlingAspect"
    }

    fn pointcut(&self) -> &PointcutExpression {
        &self.pointcut
    }

    async fn after_throwing(&self, join_point: &JoinPoint, error_msg: &str) {
        tracing::error!(
            "❌ Exception in {}: {}",
            join_point.signature(),
            error_msg
        );
    }
}

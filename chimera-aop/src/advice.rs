//! 通知（Advice）定义
//!
//! 定义了在连接点执行的各种动作

use crate::{JoinPoint, ProceedingJoinPoint};
use async_trait::async_trait;

/// 通知类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdviceType {
    /// 前置通知
    Before,
    /// 后置通知（无论成功还是失败都执行）
    After,
    /// 返回后通知（成功返回时执行）
    AfterReturning,
    /// 异常通知（抛出异常时执行）
    AfterThrowing,
    /// 环绕通知（可以控制方法执行）
    Around,
}

/// 通知 Trait
///
/// 所有通知类型的基础 trait
pub trait Advice: Send + Sync {
    /// 获取通知类型
    fn advice_type(&self) -> AdviceType;

    /// 获取通知名称
    fn name(&self) -> &str;
}

/// 前置通知 Trait
///
/// 在目标方法执行前调用
#[async_trait]
pub trait BeforeAdvice: Advice {
    /// 执行前置通知
    async fn before(&self, join_point: &JoinPoint);
}

/// 后置通知 Trait
///
/// 在目标方法执行后调用（无论成功还是失败）
#[async_trait]
pub trait AfterAdvice: Advice {
    /// 执行后置通知
    async fn after(&self, join_point: &JoinPoint);
}

/// 返回后通知 Trait
///
/// 在目标方法成功返回后调用
#[async_trait]
pub trait AfterReturningAdvice<T>: Advice {
    /// 执行返回后通知
    async fn after_returning(&self, join_point: &JoinPoint, result: &T);
}

/// 异常通知 Trait
///
/// 在目标方法抛出异常时调用
#[async_trait]
pub trait AfterThrowingAdvice: Advice {
    /// 执行异常通知
    async fn after_throwing(&self, join_point: &JoinPoint, error_msg: &str);
}

/// 环绕通知 Trait
///
/// 可以完全控制目标方法的执行
#[async_trait]
pub trait AroundAdvice<T, E>: Advice {
    /// 执行环绕通知
    async fn around(&self, pjp: ProceedingJoinPoint<T, E>) -> Result<T, E>;
}

/// 通知执行器
///
/// 负责执行各种通知
pub struct AdviceExecutor;

impl AdviceExecutor {
    /// 执行前置通知
    pub async fn execute_before(advices: &[&dyn BeforeAdvice], join_point: &JoinPoint) {
        for advice in advices {
            advice.before(join_point).await;
        }
    }

    /// 执行后置通知
    pub async fn execute_after(advices: &[&dyn AfterAdvice], join_point: &JoinPoint) {
        for advice in advices {
            advice.after(join_point).await;
        }
    }

    /// 执行返回后通知
    pub async fn execute_after_returning<T>(
        advices: &[&dyn AfterReturningAdvice<T>],
        join_point: &JoinPoint,
        result: &T,
    ) {
        for advice in advices {
            advice.after_returning(join_point, result).await;
        }
    }

    /// 执行异常通知
    pub async fn execute_after_throwing(
        advices: &[&dyn AfterThrowingAdvice],
        join_point: &JoinPoint,
        error_msg: &str,
    ) {
        for advice in advices {
            advice.after_throwing(join_point, error_msg).await;
        }
    }
}

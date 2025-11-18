//! Chimera AOP - 面向切面编程支持
//!
//! 提供类似 Spring Boot 的 AOP 功能，支持：
//! - 声明式切面定义
//! - 多种通知类型（Before、After、Around、AfterReturning、AfterThrowing）
//! - 灵活的切点表达式
//! - 编译时代码生成，运行时高性能
//! - 通过 BeanPostProcessor 自动为 Service 应用 AOP

pub mod aspect;
pub mod joinpoint;
pub mod pointcut;
pub mod registry;
pub mod advice;
pub mod error_info;
pub mod bean_post_processor;
pub mod plugin;

// 重新导出核心类型
pub use aspect::{Aspect, AspectRegistration};
pub use joinpoint::{JoinPoint, ProceedingJoinPoint};
pub use pointcut::{PointcutExpression, Pointcut};
pub use registry::{AspectRegistry, get_global_registry};
pub use advice::{Advice, AdviceType, BeforeAdvice, AfterAdvice, AroundAdvice, AfterReturningAdvice, AfterThrowingAdvice};
pub use error_info::ErrorInfo;
pub use bean_post_processor::AopBeanPostProcessor;
pub use plugin::AopPlugin;

// 导出 inventory 供宏使用
pub use inventory;

/// 预导入模块
pub mod prelude {
    pub use crate::aspect::{Aspect, AspectRegistration};
    pub use crate::joinpoint::{JoinPoint, ProceedingJoinPoint};
    pub use crate::pointcut::{PointcutExpression, Pointcut};
    pub use crate::registry::{AspectRegistry, get_global_registry};
    pub use crate::advice::*;
    pub use crate::error_info::ErrorInfo;
    pub use crate::bean_post_processor::AopBeanPostProcessor;
    pub use crate::plugin::AopPlugin;
    pub use crate::aop_execute;
}

/// 简化 AOP 方法调用的宏
///
/// 使用示例：
/// ```ignore
/// use chimera_aop::aop_execute;
///
/// pub async fn my_method(&self, id: u32) -> Result<User, Error> {
///     aop_execute!("MyService", "my_method", {
///         // your business logic here
///         Ok(self.do_something(id))
///     })
/// }
/// ```
#[macro_export]
macro_rules! aop_execute {
    ($target:expr, $method:expr, $body:block) => {
        {
            let jp = $crate::JoinPoint::new($target, $method);
            let registry = $crate::get_global_registry();
            registry.execute_with_aspects(jp, || $body).await
        }
    };
}

//! AOP BeanPostProcessor - 自动为 Service Bean 应用 AOP 功能
//!
//! 通过实现 BeanPostProcessor，在 Bean 初始化后自动为符合条件的 Bean 包装 AOP 代理

use chimera_core::prelude::*;
use std::any::Any;
use std::sync::Arc;

/// AOP BeanPostProcessor
///
/// 在 Bean 初始化后，自动为符合条件的 Bean（如 Service）包装 AOP 代理
///
/// ## 工作原理
///
/// 1. 在 Bean 初始化后检查 Bean 是否匹配任何切面的切点表达式
/// 2. 如果匹配，则为 Bean 创建 AOP 代理包装
/// 3. 代理会拦截方法调用并应用切面逻辑
///
/// ## 限制
///
/// 由于 Rust 没有运行时反射和动态代理，当前实现主要用于演示 BeanPostProcessor 机制。
/// 实际的 AOP 代理创建需要通过过程宏在编译时生成代理代码。
///
/// ## 使用示例
///
/// ```ignore
/// use chimera_aop::AopBeanPostProcessor;
///
/// // 在应用启动时注册
/// context.add_bean_post_processor(Arc::new(AopBeanPostProcessor::new())).await;
/// ```
pub struct AopBeanPostProcessor {
    /// 是否启用 AOP
    enabled: bool,
}

impl AopBeanPostProcessor {
    /// 创建新的 AOP BeanPostProcessor
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// 创建禁用的 AOP BeanPostProcessor
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// 检查 Bean 名称是否应该应用 AOP
    ///
    /// 默认策略：Bean 名称以 "Service" 结尾的会应用 AOP
    fn should_apply_aop(&self, bean_name: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // 检查是否是 Service 类型的 Bean
        bean_name.ends_with("Service")
    }

    /// 检查 Bean 是否匹配任何已注册的切面
    fn matches_any_aspect(&self, bean_name: &str) -> bool {
        let registry = crate::get_global_registry();

        // 简单的检查：如果有切面注册，且 bean 名称符合条件，则认为匹配
        !registry.is_empty() && self.should_apply_aop(bean_name)
    }
}

impl Default for AopBeanPostProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl BeanPostProcessor for AopBeanPostProcessor {
    fn name(&self) -> &str {
        "AopBeanPostProcessor"
    }

    fn order(&self) -> i32 {
        // AOP 处理器应该在其他处理器之后执行
        // 这样可以确保 Bean 已经完全初始化
        2000
    }

    fn post_process_after_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        bean_name: &str,
    ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        // 检查是否应该应用 AOP
        if !self.matches_any_aspect(bean_name) {
            tracing::trace!("Bean '{}' does not match any aspect, skipping AOP wrapping", bean_name);
            return Ok(bean);
        }

        tracing::info!("🔷 [AOP-BeanPostProcessor] Bean '{}' matches AOP pointcuts", bean_name);

        // TODO: 在这里创建 AOP 代理包装
        // 由于 Rust 的限制，我们不能像 Java 那样在运行时创建动态代理
        // 实际的代理需要通过过程宏在编译时生成
        //
        // 当前实现仅记录日志，表明 BeanPostProcessor 机制正常工作
        //
        // 未来的增强方向：
        // 1. 使用过程宏在编译时生成代理类
        // 2. 在 #[Component] 宏中自动生成代理版本
        // 3. 通过 trait object 实现有限的动态代理

        tracing::info!(
            "🔷 [AOP-BeanPostProcessor] Would wrap '{}' with AOP proxy (proxy generation not yet implemented)",
            bean_name
        );

        // 目前直接返回原始 Bean
        // 未来这里会返回 AOP 代理包装的 Bean
        Ok(bean)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_apply_aop() {
        let processor = AopBeanPostProcessor::new();

        // 应该应用 AOP 的 Bean
        assert!(processor.should_apply_aop("userService"));
        assert!(processor.should_apply_aop("orderService"));
        assert!(processor.should_apply_aop("paymentService"));

        // 不应该应用 AOP 的 Bean
        assert!(!processor.should_apply_aop("userController"));
        assert!(!processor.should_apply_aop("appConfig"));
        assert!(!processor.should_apply_aop("repository"));
    }

    #[test]
    fn test_disabled_processor() {
        let processor = AopBeanPostProcessor::disabled();

        // 禁用的处理器不应该应用 AOP
        assert!(!processor.should_apply_aop("userService"));
        assert!(!processor.should_apply_aop("orderService"));
    }

    #[test]
    fn test_processor_order() {
        let processor = AopBeanPostProcessor::new();

        // AOP 处理器应该有较高的 order 值（较低的优先级）
        // 这样可以确保在其他处理器之后执行
        assert_eq!(processor.order(), 2000);
    }
}

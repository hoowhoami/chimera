//! BeanPostProcessor - Bean 工厂扩展机制
//!
//! 提供在 Bean 初始化前后进行自定义处理的钩子，类似 Spring 的 BeanPostProcessor

use std::any::Any;
use std::sync::Arc;
use crate::error::ContainerResult;
use crate::ApplicationContext;
use std::future::Future;
use std::pin::Pin;

/// BeanPostProcessor 获取函数类型
pub type BeanPostProcessorGetter = fn(&Arc<ApplicationContext>) -> Pin<Box<dyn Future<Output = ContainerResult<Arc<dyn BeanPostProcessor>>> + Send>>;

/// BeanPostProcessor 标记 - 用于 inventory 收集
///
/// 标记哪些 Component 实现了 BeanPostProcessor trait
pub struct BeanPostProcessorMarker {
    pub bean_name: &'static str,
    pub type_name: &'static str,
    pub getter: BeanPostProcessorGetter,
}

inventory::collect!(BeanPostProcessorMarker);

/// BeanPostProcessor trait
///
/// 在 Bean 初始化的不同阶段提供钩子，允许自定义修改 Bean 实例
///
/// 使用场景：
/// - AOP 代理创建
/// - Bean 包装
/// - 属性注入增强
/// - 验证等
///
/// # 示例
///
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_core_macros::{BeanPostProcessor, Component};
///
/// #[derive(BeanPostProcessor, Component)]
/// pub struct LoggingBeanPostProcessor {
///     #[autowired]
///     app_context: Arc<ApplicationContext>,
/// }
///
/// impl BeanPostProcessor for LoggingBeanPostProcessor {
///     fn post_process_before_initialization(
///         &self,
///         bean: Arc<dyn Any + Send + Sync>,
///         bean_name: &str
///     ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
///         tracing::info!("Before initialization: {}", bean_name);
///         Ok(bean)
///     }
///
///     fn post_process_after_initialization(
///         &self,
///         bean: Arc<dyn Any + Send + Sync>,
///         bean_name: &str
///     ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
///         tracing::info!("After initialization: {}", bean_name);
///         Ok(bean)
///     }
/// }
/// ```
pub trait BeanPostProcessor: Send + Sync {
    /// 在 Bean 初始化回调（init）之前调用
    ///
    /// # 参数
    /// - `bean`: Bean 实例
    /// - `bean_name`: Bean 的名称
    ///
    /// # 返回
    /// 返回处理后的 Bean 实例（可以是原始 Bean，也可以是包装后的 Bean）
    fn post_process_before_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        _bean_name: &str,
    ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        // 默认实现：直接返回原始 Bean
        Ok(bean)
    }

    /// 在 Bean 初始化回调（init）之后调用
    ///
    /// # 参数
    /// - `bean`: Bean 实例（已经过 init 回调）
    /// - `bean_name`: Bean 的名称
    ///
    /// # 返回
    /// 返回处理后的 Bean 实例（可以是原始 Bean，也可以是包装后的 Bean）
    ///
    /// # 典型用途
    /// - 创建 AOP 代理
    /// - 包装 Bean
    /// - 注入额外的依赖
    fn post_process_after_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        _bean_name: &str,
    ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        // 默认实现：直接返回原始 Bean
        Ok(bean)
    }

    /// 获取处理器的名称（用于日志和调试）
    fn name(&self) -> &str {
        "BeanPostProcessor"
    }

    /// 获取处理器的优先级（数字越小优先级越高）
    ///
    /// 默认为 1000，可以通过重写此方法来调整优先级
    fn order(&self) -> i32 {
        1000
    }
}

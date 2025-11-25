//! Bean 生命周期接口
//!
//! 提供 Spring 风格的 Bean 生命周期管理接口

use crate::error::Result;
use crate::ApplicationContext;
use std::any::Any;
use std::sync::Arc;

/// BeanFactoryPostProcessor 获取函数类型
pub type BeanFactoryPostProcessorGetter =
    fn(&crate::ApplicationContext) -> Result<Arc<dyn BeanFactoryPostProcessor>>;

/// BeanFactoryPostProcessor 标记 - 用于 inventory 收集
///
/// 标记哪些 Component 实现了 BeanFactoryPostProcessor trait
pub struct BeanFactoryPostProcessorMarker {
    pub bean_name: &'static str,
    pub type_name: &'static str,
    pub getter: BeanFactoryPostProcessorGetter,
}

inventory::collect!(BeanFactoryPostProcessorMarker);

/// BeanFactoryPostProcessor - Bean 工厂后置处理器
///
/// 在 Bean 定义加载后、Bean 实例化之前执行
/// 可以修改 Bean 定义元数据
///
/// # Spring 语义
/// - 在容器启动阶段执行
/// - 在任何 Bean 实例化之前执行（除了 BeanFactoryPostProcessor 自身）
/// - 可以修改 Bean 定义
/// - 作用于 Bean 定义元数据，而不是 Bean 实例
///
/// # 示例
///
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_core_macros::{BeanFactoryPostProcessor, Component};
///
/// #[derive(BeanFactoryPostProcessor, Component)]
/// pub struct CustomBeanFactoryPostProcessor;
///
/// impl BeanFactoryPostProcessor for CustomBeanFactoryPostProcessor {
///     fn post_process_bean_factory(&self, context: &ApplicationContext) -> Result<()> {
///         // 获取 BeanFactory
///         let bean_factory = context.get_bean_factory();
///
///         // 获取所有 Bean 定义
///         let definitions = bean_factory.get_bean_definitions();
///         tracing::info!("Total bean definitions: {}", definitions.len());
///
///         // 修改 Bean 定义
///         // bean_factory.register_bean_definition(...);
///
///         Ok(())
///     }
/// }
/// ```
pub trait BeanFactoryPostProcessor: Send + Sync {
    /// 在 Bean 工厂标准初始化之后修改应用上下文的内部 Bean 工厂
    ///
    /// # 参数
    /// - `context`: ApplicationContext 引用，可以通过 context.get_bean_factory() 访问和修改 Bean 定义
    ///
    /// # 返回
    /// 成功返回 Ok(())，失败返回错误
    fn post_process_bean_factory(&self, context: &crate::ApplicationContext) -> Result<()>;

    /// 获取处理器的优先级（数字越小优先级越高）
    ///
    /// 默认为 1000
    fn order(&self) -> i32 {
        1000
    }
}

/// BeanPostProcessor 获取函数类型
pub type BeanPostProcessorGetter = fn(&Arc<ApplicationContext>) -> Result<Arc<dyn BeanPostProcessor>>;

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
///     ) -> Result<Arc<dyn Any + Send + Sync>> {
///         tracing::info!("Before initialization: {}", bean_name);
///         Ok(bean)
///     }
///
///     fn post_process_after_initialization(
///         &self,
///         bean: Arc<dyn Any + Send + Sync>,
///         bean_name: &str
///     ) -> Result<Arc<dyn Any + Send + Sync>> {
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
    ) -> Result<Arc<dyn Any + Send + Sync>> {
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
    ) -> Result<Arc<dyn Any + Send + Sync>> {
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

/// SmartInitializingSingleton 获取函数类型
pub type SmartInitializingSingletonGetter =
    fn(&Arc<ApplicationContext>) -> Result<Arc<dyn SmartInitializingSingleton>>;

/// SmartInitializingSingleton 标记 - 用于 inventory 收集
///
/// 标记哪些 Component 实现了 SmartInitializingSingleton trait
pub struct SmartInitializingSingletonMarker {
    pub bean_name: &'static str,
    pub type_name: &'static str,
    pub getter: SmartInitializingSingletonGetter,
}

inventory::collect!(SmartInitializingSingletonMarker);

/// SmartInitializingSingleton - 智能初始化单例接口
///
/// 在所有单例 Bean 初始化完成后调用
///
/// # Spring 语义
/// - 在所有非延迟加载的单例 Bean 实例化和初始化完成后调用
/// - 在常规单例实例化阶段结束时调用
/// - 适用于需要在所有 Bean 就绪后执行的逻辑
/// - 不会触发 Bean 的提前实例化
///
/// # 示例
///
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_core_macros::Component;
///
/// #[derive(Component)]
/// pub struct StartupService;
///
/// impl SmartInitializingSingleton for StartupService {
///     fn after_singletons_instantiated(&self) -> Result<()> {
///         // 在所有单例 Bean 初始化完成后执行
///         tracing::info!("All singletons initialized, starting background tasks");
///         Ok(())
///     }
/// }
/// ```
pub trait SmartInitializingSingleton: Send + Sync {
    /// 在所有单例 Bean 实例化完成后调用
    ///
    /// 此回调在单例预实例化阶段结束时触发
    ///
    /// # 返回
    /// 成功返回 Ok(())，失败返回错误
    fn after_singletons_instantiated(&self) -> Result<()>;
}

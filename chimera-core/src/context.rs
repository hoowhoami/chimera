use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::bean_factory::{DefaultListableBeanFactory, BeanFactory, BeanFactoryExt, ListableBeanFactory, ConfigurableBeanFactory, ConfigurableListableBeanFactory};
use crate::lifecycle::BeanPostProcessor;
use crate::constants;
use crate::{
    bean::{BeanDefinition, FunctionFactory},
    config::Environment,
    error::{ContainerError, ContainerResult},
    event::{ApplicationEventPublisher, ApplicationShutdownEvent, Event, EventListener},
    Scope,
};

/// Shutdown hook类型
pub type ShutdownHook = Box<dyn Fn() -> ContainerResult<()> + Send + Sync>;

/// 容器 trait - 定义依赖注入容器的核心接口
pub trait Container: Send + Sync {
    /// 注册 Bean 定义
    fn register(&self, definition: BeanDefinition) -> ContainerResult<()>;

    /// 通过名称获取 Bean
    fn get_bean(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>>;

    /// 通过类型获取 Bean
    fn get_bean_by_type<T: Any + Send + Sync>(&self) -> ContainerResult<Arc<T>>;

    /// 检查是否包含指定名称的 Bean
    fn contains_bean(&self, name: &str) -> bool;

    /// 检查是否包含指定类型的 Bean
    fn contains_bean_by_type<T: Any + Send + Sync>(&self) -> bool;

    /// 获取所有 Bean 的名称
    fn get_bean_names(&self) -> Vec<String>;
}

/// 应用上下文 - Container 的默认实现
///
/// ApplicationContext 是框架的核心，持有 BeanFactory、Environment 和 EventPublisher
pub struct ApplicationContext {
    /// Bean 工厂 - 负责 Bean 的创建和管理
    bean_factory: Arc<DefaultListableBeanFactory>,

    /// 配置环境
    environment: Arc<Environment>,

    /// 事件发布器
    event_publisher: Arc<ApplicationEventPublisher>,

    /// Shutdown hooks
    shutdown_hooks: RwLock<Vec<ShutdownHook>>,

    /// 应用名称（用于事件）
    app_name: RwLock<Option<String>>,

    /// Bean 工厂后置处理器列表（按优先级排序）
    bean_factory_post_processors: RwLock<Vec<Arc<dyn crate::lifecycle::BeanFactoryPostProcessor>>>,
}

impl ApplicationContext {
    /// 创建新的应用上下文（使用默认的同步事件处理）
    pub fn new() -> Self {
        Self::new_with_async_events(false)
    }

    /// 创建新的应用上下文，指定是否异步处理事件
    pub fn new_with_async_events(async_events: bool) -> Self {
        Self::new_with_async_events_and_env(async_events, None)
    }

    /// 创建新的应用上下文，指定是否异步处理事件和可选的Environment
    ///
    /// 此方法主要供 ApplicationContextBuilder 内部使用
    pub(crate) fn new_with_async_events_and_env(
        async_events: bool,
        environment: Option<Arc<Environment>>,
    ) -> Self {
        use crate::event::{ApplicationEventPublisher, SimpleApplicationEventMulticaster};

        // 根据配置创建相应的 multicaster
        let multicaster: Arc<dyn crate::event::ApplicationEventMulticaster> = if async_events {
            Arc::new(SimpleApplicationEventMulticaster::new_async())
        } else {
            Arc::new(SimpleApplicationEventMulticaster::new())
        };

        Self {
            bean_factory: Arc::new(DefaultListableBeanFactory::new()),
            environment: environment.unwrap_or_else(|| Arc::new(Environment::new())),
            event_publisher: Arc::new(ApplicationEventPublisher::new(multicaster)),
            shutdown_hooks: RwLock::new(Vec::new()),
            app_name: RwLock::new(None),
            bean_factory_post_processors: RwLock::new(Vec::new()),
        }
    }

    /// 获取内部的 BeanFactory（用于 BeanFactoryPostProcessor）
    pub fn get_bean_factory(&self) -> &Arc<DefaultListableBeanFactory> {
        &self.bean_factory
    }

    /// 设置应用名称
    pub fn set_app_name(&self, name: String) {
        let mut app_name = self.app_name.write();
        *app_name = Some(name);
    }

    /// 获取应用名称
    pub fn get_app_name(&self) -> Option<String> {
        self.app_name.read().clone()
    }

    /// 获取 Environment
    pub fn environment(&self) -> &Arc<Environment> {
        &self.environment
    }

    /// 获取 Environment（别名，方便使用）
    pub fn get_environment(&self) -> &Arc<Environment> {
        &self.environment
    }

    /// 获取 EventPublisher
    pub fn event_publisher(&self) -> &Arc<ApplicationEventPublisher> {
        &self.event_publisher
    }

    /// 获取 EventPublisher（别名，方便使用）
    pub fn get_event_publisher(&self) -> &Arc<ApplicationEventPublisher> {
        &self.event_publisher
    }

    /// 发布事件
    pub fn publish_event(&self, event: Arc<dyn Event>) {
        self.event_publisher.publish_event(event);
    }

    /// 注册事件监听器
    pub fn register_listener(&self, listener: Arc<dyn EventListener>) {
        self.event_publisher.add_listener(listener);
    }

    /// 注册 shutdown hook
    ///
    /// Shutdown hook 会在应用关闭时按注册顺序执行
    pub fn register_shutdown_hook<F>(&self, hook: F)
    where
        F: Fn() -> ContainerResult<()> + Send + Sync + 'static,
    {
        let mut hooks = self.shutdown_hooks.write();
        hooks.push(Box::new(hook));
        tracing::debug!("Registered shutdown hook, total: {}", hooks.len());
    }

    /// 注册 BeanPostProcessor
    ///
    /// BeanPostProcessor 会在 Bean 初始化前后进行处理，按优先级顺序执行
    pub fn add_bean_post_processor(&self, processor: Arc<dyn BeanPostProcessor>) {
        // 委托给 BeanFactory
        self.bean_factory.add_bean_post_processor(processor);
    }

    /// 扫描并注册所有通过 #[derive(BeanPostProcessor)] 宏标记的处理器
    ///
    /// 此方法会自动从容器中获取所有 BeanPostProcessor 实例
    /// 注意：BeanPostProcessor 必须同时使用 #[derive(Component)] 注册为组件
    pub fn scan_bean_post_processors(self: &Arc<Self>) {
        use crate::lifecycle::BeanPostProcessorMarker;

        let markers: Vec<_> = inventory::iter::<BeanPostProcessorMarker>().collect();

        if markers.is_empty() {
            tracing::debug!("No BeanPostProcessor markers found");
            return;
        }

        tracing::info!("Starting BeanPostProcessor scan, found {} marker(s)", markers.len());

        for marker in markers {
            tracing::debug!("  ├─ Looking for BeanPostProcessor: {} ({})", marker.bean_name, marker.type_name);

            // 使用 getter 函数从容器中获取 BeanPostProcessor 实例
            match (marker.getter)(self) {
                Ok(processor) => {
                    tracing::debug!("  ├─ Successfully retrieved BeanPostProcessor: {}", marker.bean_name);
                    self.add_bean_post_processor(processor);
                }
                Err(e) => {
                    tracing::error!("  ├─ Failed to get BeanPostProcessor '{}' from container: {}", marker.bean_name, e);
                    tracing::error!("  └─ Make sure {} is annotated with #[derive(Component)]", marker.type_name);
                }
            }
        }

        let count = self.bean_factory.get_bean_post_processors().len();
        tracing::info!("BeanPostProcessor scan completed, registered {} processor(s)", count);
    }

    /// 注册 BeanFactoryPostProcessor
    ///
    /// BeanFactoryPostProcessor 会在 Bean 定义加载后、Bean 实例化之前执行，按优先级顺序执行
    pub fn add_bean_factory_post_processor(
        &self,
        processor: Arc<dyn crate::lifecycle::BeanFactoryPostProcessor>,
    ) {
        let mut processors = self.bean_factory_post_processors.write();
        processors.push(processor);
        // 按优先级排序（order 越小优先级越高）
        processors.sort_by_key(|p| p.order());
        tracing::debug!(
            "Registered BeanFactoryPostProcessor with order {}",
            processors.last().unwrap().order()
        );
    }

    /// 扫描并注册所有实现了 BeanFactoryPostProcessor 的 Bean
    ///
    /// 此方法会自动从容器中获取所有 BeanFactoryPostProcessor 实例
    /// 注意：BeanFactoryPostProcessor 必须同时使用 #[derive(Component)] 注册为组件
    pub fn scan_bean_factory_post_processors(self: &Arc<Self>) {
        use crate::lifecycle::BeanFactoryPostProcessorMarker;

        let markers: Vec<_> = inventory::iter::<BeanFactoryPostProcessorMarker>().collect();

        if markers.is_empty() {
            tracing::debug!("No BeanFactoryPostProcessor markers found");
            return;
        }

        tracing::info!(
            "Starting BeanFactoryPostProcessor scan, found {} marker(s)",
            markers.len()
        );

        for marker in markers {
            tracing::debug!(
                "  ├─ Looking for BeanFactoryPostProcessor: {} ({})",
                marker.bean_name,
                marker.type_name
            );

            // 使用 getter 函数从容器中获取 BeanFactoryPostProcessor 实例
            match (marker.getter)(self) {
                Ok(processor) => {
                    tracing::debug!(
                        "  ├─ Successfully retrieved BeanFactoryPostProcessor: {}",
                        marker.bean_name
                    );
                    self.add_bean_factory_post_processor(processor);
                }
                Err(e) => {
                    tracing::error!(
                        "  ├─ Failed to get BeanFactoryPostProcessor '{}' from container: {}",
                        marker.bean_name,
                        e
                    );
                    tracing::error!(
                        "  └─ Make sure {} is annotated with #[derive(Component)]",
                        marker.type_name
                    );
                }
            }
        }

        let count = self.bean_factory_post_processors.read().len();
        tracing::info!(
            "BeanFactoryPostProcessor scan completed, registered {} processor(s)",
            count
        );
    }

    /// 调用所有 BeanFactoryPostProcessor
    ///
    /// 在 Bean 定义加载后、Bean 实例化之前调用
    /// 按照 Spring 语义，这应该在组件扫描之后、Bean 初始化之前执行
    pub fn invoke_bean_factory_post_processors(self: &Arc<Self>) -> ContainerResult<()> {
        let processors = self.bean_factory_post_processors.read();

        if processors.is_empty() {
            tracing::debug!("No BeanFactoryPostProcessors to invoke");
            return Ok(());
        }

        tracing::info!("Invoking {} BeanFactoryPostProcessor(s)", processors.len());

        for processor in processors.iter() {
            processor.post_process_bean_factory(self).map_err(|e| {
                ContainerError::BeanCreationFailed(format!(
                    "BeanFactoryPostProcessor failed: {}",
                    e
                ))
            })?;
        }

        tracing::info!("All BeanFactoryPostProcessors invoked successfully");
        Ok(())
    }


    /// 构建器模式创建上下文
    pub fn builder() -> ApplicationContextBuilder {
        ApplicationContextBuilder::new()
    }

    /// 注册 Bean
    pub fn register_bean<T, F>(
        &self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<()>
    where
        T: Any + Send + Sync,
        F: Fn() -> ContainerResult<T> + Send + Sync + 'static,
    {
        let name = name.into();
        let definition = BeanDefinition::new(name.clone(), FunctionFactory::new(factory));
        self.register(definition)
    }

    /// 注册单例 Bean
    pub fn register_singleton<T, F>(
        &self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<()>
    where
        T: Any + Send + Sync,
        F: Fn() -> ContainerResult<T> + Send + Sync + 'static,
    {
        let name = name.into();
        let definition = BeanDefinition::new(name.clone(), FunctionFactory::new(factory))
            .with_scope(Scope::Singleton);
        self.register(definition)
    }

    /// 注册原型 Bean
    pub fn register_prototype<T, F>(
        &self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<()>
    where
        T: Any + Send + Sync,
        F: Fn() -> ContainerResult<T> + Send + Sync + 'static,
    {
        let name = name.into();
        let definition = BeanDefinition::new(name.clone(), FunctionFactory::new(factory))
            .with_scope(Scope::Prototype);
        self.register(definition)
    }

    /// 初始化所有非延迟加载的单例 Bean
    pub fn initialize(self: &Arc<Self>) -> ContainerResult<()> {

        // 委托给 BeanFactory 进行预实例化
        use crate::bean_factory::ConfigurableListableBeanFactory;
        self.bean_factory.preinstantiate_singletons()?;

        // 所有单例 Bean 初始化完成后，调用 SmartInitializingSingleton.after_singletons_instantiated
        tracing::debug!("All singleton beans initialized, calling SmartInitializingSingleton callbacks");
        self.invoke_smart_initializing_singletons()?;

        Ok(())
    }

    /// 调用所有实现了 SmartInitializingSingleton 的 Bean 的回调
    ///
    /// 在所有非延迟加载的单例 Bean 初始化完成后调用
    ///
    /// 通过 inventory 机制自动收集所有使用 #[derive(SmartInitializingSingleton)] 标记的 Bean
    fn invoke_smart_initializing_singletons(self: &Arc<Self>) -> ContainerResult<()> {
        use crate::lifecycle::SmartInitializingSingletonMarker;

        let markers: Vec<_> = inventory::iter::<SmartInitializingSingletonMarker>().collect();

        if markers.is_empty() {
            tracing::debug!("No SmartInitializingSingleton markers found");
            return Ok(());
        }

        let marker_count = markers.len();
        tracing::info!(
            "Invoking SmartInitializingSingleton callbacks, found {} marker(s)",
            marker_count
        );

        for marker in markers {
            tracing::debug!(
                "  ├─ Invoking SmartInitializingSingleton: {} ({})",
                marker.bean_name,
                marker.type_name
            );

            // 使用 getter 函数从容器中获取 SmartInitializingSingleton 实例
            match (marker.getter)(self) {
                Ok(singleton) => {
                    // 调用 after_singletons_instantiated 回调
                    if let Err(e) = singleton.after_singletons_instantiated() {
                        tracing::error!(
                            "  ├─ SmartInitializingSingleton '{}' callback failed: {}",
                            marker.bean_name,
                            e
                        );
                        return Err(e);
                    }
                    tracing::debug!(
                        "  ├─ Successfully invoked SmartInitializingSingleton: {}",
                        marker.bean_name
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "  ├─ Failed to get SmartInitializingSingleton '{}' from container: {}",
                        marker.bean_name,
                        e
                    );
                    tracing::error!(
                        "  └─ Make sure {} is annotated with #[derive(Component)]",
                        marker.type_name
                    );
                    return Err(e);
                }
            }
        }

        tracing::info!(
            "SmartInitializingSingleton callbacks completed, invoked {} callback(s)",
            marker_count
        );
        Ok(())
    }

    /// 验证所有 Bean 的依赖关系
    ///
    /// 检查：
    /// - 缺失的依赖（声明的依赖没有注册）
    /// - 循环依赖（A -> B -> C -> A）
    ///
    /// 建议在 `scan_components()` 或 `initialize()` 之后调用此方法
    pub fn validate_dependencies(&self) -> ContainerResult<()> {
        use crate::utils::dependency::validate_dependency_graph;

        // 从 BeanFactory 获取依赖图
        let dependency_map = self.bean_factory.get_bean_definitions();

        // 验证依赖图
        validate_dependency_graph(&dependency_map)
            .map_err(|e| ContainerError::DependencyValidationFailed(e.to_string()))?;

        tracing::info!(
            "Dependency validation passed for {} bean(s)",
            dependency_map.len()
        );

        Ok(())
    }


    /// 销毁所有单例 Bean（调用 destroy 回调）
    /// 注意：只有当 Arc 的引用计数为 1 时才能调用 destroy
    pub fn shutdown(&self) -> ContainerResult<()> {
        tracing::info!("Starting application shutdown");

        // 1. 发布 ApplicationShutdownEvent
        let app_name = self
            .get_app_name()
            .unwrap_or_else(|| "Application".to_string());
        let shutdown_event = Arc::new(ApplicationShutdownEvent::new(app_name));
        self.publish_event(shutdown_event);

        // 2. 执行所有 shutdown hooks
        let hooks = self.shutdown_hooks.read();
        tracing::info!("Executing {} shutdown hook(s)", hooks.len());
        for (idx, hook) in hooks.iter().enumerate() {
            match hook() {
                Ok(_) => tracing::debug!("Shutdown hook {} executed successfully", idx + 1),
                Err(e) => tracing::warn!("Shutdown hook {} failed: {}", idx + 1, e),
            }
        }
        drop(hooks); // 释放读锁

        // 3. 销毁所有 beans
        use crate::bean_factory::ConfigurableListableBeanFactory;
        self.bean_factory.destroy_singletons()?;

        tracing::info!("Application shutdown complete");
        Ok(())
    }
}

impl Default for ApplicationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Container for ApplicationContext {
    fn register(&self, definition: BeanDefinition) -> ContainerResult<()> {
        // 委托给 BeanFactory
        self.bean_factory.register_bean_definition(definition.name.clone(), definition)
    }

    fn get_bean(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        // 委托给 BeanFactory
        self.bean_factory.get_bean(name)
    }

    fn get_bean_by_type<T: Any + Send + Sync>(&self) -> ContainerResult<Arc<T>> {
        // 委托给 BeanFactory
        self.bean_factory.get_bean_by_type::<T>()
    }

    fn contains_bean(&self, name: &str) -> bool {
        // 委托给 BeanFactory
        self.bean_factory.contains_bean(name)
    }

    fn contains_bean_by_type<T: Any + Send + Sync>(&self) -> bool {
        // 委托给 BeanFactory
        self.bean_factory.contains_bean_by_type::<T>()
    }

    fn get_bean_names(&self) -> Vec<String> {
        // 委托给 BeanFactory
        self.bean_factory.get_bean_names()
    }
}

impl ApplicationContext {
    /// 导出所有 bean 定义和类型映射（内部方法，供 ApplicationContextBuilder 使用）
    ///
    /// 注意：此方法会清空当前 context 的定义，应该只在转移所有权时调用
    pub(crate) fn export_definitions(&self) -> (HashMap<String, BeanDefinition>, HashMap<TypeId, String>) {
        // 由于 BeanFactory 没有提供导出方法，这里暂时返回空的 HashMap
        // 在实际使用中，可能需要重新设计这个方法
        (HashMap::new(), HashMap::new())
    }

    /// 导入 bean 定义和类型映射（内部方法，供 ApplicationContextBuilder 使用）
    pub(crate) fn import_definitions(&self, _definitions: HashMap<String, BeanDefinition>, _type_to_name: HashMap<TypeId, String>) {
        // 由于 BeanFactory 没有提供导入方法，这里暂时不做任何操作
        // 在实际使用中，可能需要重新设计这个方法
    }
}

/// 应用上下文构建器
pub struct ApplicationContextBuilder {
    context: ApplicationContext,
    async_events: bool,
}

impl ApplicationContextBuilder {
    pub fn new() -> Self {
        Self {
            context: ApplicationContext::new(),
            async_events: false,
        }
    }

    /// 设置是否异步处理事件
    ///
    /// 默认为 false（同步处理）
    /// 设置为 true 时，事件将在独立的 tokio 任务中异步处理
    pub fn async_events(mut self, async_events: bool) -> Self {
        self.async_events = async_events;
        self
    }

    /// 注册 Bean
    pub fn register(self, definition: BeanDefinition) -> ContainerResult<Self> {
        self.context.register(definition)?;
        Ok(self)
    }

    /// 注册单例 Bean
    pub fn register_singleton<T, F>(
        self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<Self>
    where
        T: Any + Send + Sync,
        F: Fn() -> ContainerResult<T> + Send + Sync + 'static,
    {
        self.context.register_singleton(name, factory)?;
        Ok(self)
    }

    /// 注册原型 Bean
    pub fn register_prototype<T, F>(
        self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<Self>
    where
        T: Any + Send + Sync,
        F: Fn() -> ContainerResult<T> + Send + Sync + 'static,
    {
        self.context.register_prototype(name, factory)?;
        Ok(self)
    }

    /// 添加配置源到 Environment
    pub fn add_property_source(self, source: Box<dyn crate::PropertySource>) -> Self {
        self.context.environment.add_property_source(source);
        self
    }

    /// 添加配置源（可变引用版本，不消费 self）
    pub(crate) fn add_property_source_mut(&mut self, source: Box<dyn crate::PropertySource>) {
        self.context.environment.add_property_source(source);
    }

    /// 设置激活的 profiles
    pub fn set_active_profiles(self, profiles: Vec<String>) -> Self {
        self.context.environment.set_active_profiles(profiles);
        self
    }

    /// 构建上下文
    pub fn build(self) -> ContainerResult<Arc<ApplicationContext>> {
        // 根据 async_events 设置创建最终的 ApplicationContext
        let context = if self.async_events {
            // 需要异步事件处理，创建新的 context 并复制配置
            tracing::debug!("Building ApplicationContext with async event processing");

            // 1. 提取当前 context 的环境和 bean 定义
            let environment = Arc::clone(&self.context.environment);
            let (definitions, type_to_name) = self.context.export_definitions();

            // 2. 创建新的 ApplicationContext，使用异步事件处理和现有环境
            let new_context = ApplicationContext::new_with_async_events_and_env(true, Some(environment));

            // 3. 导入 bean 定义
            new_context.import_definitions(definitions, type_to_name);

            Arc::new(new_context)
        } else {
            // 使用默认的同步事件处理
            Arc::new(self.context)
        };

        // 注意：核心组件注册已移到 initialize() 方法中
        // 这样可以确保所有用户组件都注册完成后再注册核心组件
        Ok(context)
    }
}

impl Default for ApplicationContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

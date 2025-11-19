use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::bean_post_processor::BeanPostProcessor;
use crate::constants;
use crate::{
    bean::{BeanDefinition, FunctionFactory},
    config::Environment,
    error::{ContainerError, ContainerResult},
    event::{ApplicationEventPublisher, ApplicationShutdownEvent, Event, EventListener},
    utils::dependency::CreationTracker,
    Scope,
};

/// Shutdown hook类型
pub type ShutdownHook = Box<dyn Fn() -> ContainerResult<()> + Send + Sync>;

/// 标识框架核心组件的 trait
///
/// 实现此 trait 的类型会被 @autowired 宏识别为核心组件，
/// 并使用特殊的注入方式而不是通过容器查找
pub trait CoreComponent: Send + Sync {
    /// 获取核心组件在容器中的Bean名称
    fn core_bean_name() -> &'static str;

    /// 从ApplicationContext直接获取核心组件实例
    fn get_from_context(context: &Arc<ApplicationContext>) -> Arc<Self>;
}

/// 容器 trait - 定义依赖注入容器的核心接口（同步版本）
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
pub struct ApplicationContext {
    /// Bean 定义存储（使用 parking_lot::RwLock，性能更好）
    definitions: RwLock<HashMap<String, BeanDefinition>>,

    /// 单例 Bean 缓存
    singletons: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,

    /// 类型到名称的映射
    type_to_name: RwLock<HashMap<TypeId, String>>,

    /// 循环依赖检测 - 跟踪正在创建的 Bean
    creation_tracker: CreationTracker,

    /// 配置环境
    environment: Arc<Environment>,

    /// 事件发布器
    event_publisher: Arc<ApplicationEventPublisher>,

    /// Shutdown hooks
    shutdown_hooks: RwLock<Vec<ShutdownHook>>,

    /// 应用名称（用于事件）
    app_name: RwLock<Option<String>>,

    /// Bean 后置处理器列表（按优先级排序）
    bean_post_processors: RwLock<Vec<Arc<dyn BeanPostProcessor>>>,
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
            definitions: RwLock::new(HashMap::new()),
            singletons: RwLock::new(HashMap::new()),
            type_to_name: RwLock::new(HashMap::new()),
            creation_tracker: CreationTracker::new(),
            environment: environment.unwrap_or_else(|| Arc::new(Environment::new())),
            event_publisher: Arc::new(ApplicationEventPublisher::new(multicaster)),
            shutdown_hooks: RwLock::new(Vec::new()),
            app_name: RwLock::new(None),
            bean_post_processors: RwLock::new(Vec::new()),
        }
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

    /// 发布事件（同步方式）
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
        let mut processors = self.bean_post_processors.write();
        processors.push(processor);
        // 按优先级排序（order 越小优先级越高）
        processors.sort_by_key(|p| p.order());
        tracing::debug!("Registered BeanPostProcessor: '{}' with order {}",
            processors.last().unwrap().name(),
            processors.last().unwrap().order());
    }

    /// 扫描并注册所有通过 #[derive(BeanPostProcessor)] 宏标记的处理器
    ///
    /// 此方法会自动从容器中获取所有 BeanPostProcessor 实例
    /// 注意：BeanPostProcessor 必须同时使用 #[derive(Component)] 注册为组件
    pub fn scan_bean_post_processors(self: &Arc<Self>) {
        use crate::bean_post_processor::BeanPostProcessorMarker;

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

        let count = self.bean_post_processors.read().len();
        tracing::info!("BeanPostProcessor scan completed, registered {} processor(s)", count);
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
    pub fn initialize(&self) -> ContainerResult<()> {
        use crate::utils::dependency::topological_sort;

        // 获取所有需要初始化的 bean 及其依赖关系
        let (beans_to_init, dependency_map) = {
            let definitions = self.definitions.read();

            let mut beans = Vec::new();
            let mut deps = HashMap::new();

            for (name, definition) in definitions.iter() {
                if definition.scope == Scope::Singleton && !definition.lazy {
                    beans.push(name.clone());
                    deps.insert(name.clone(), definition.dependencies.clone());
                }
            }

            (beans, deps)
        };

        if beans_to_init.is_empty() {
            return Ok(());
        }

        // 拓扑排序，确定初始化顺序
        let sorted_beans =
            topological_sort(&dependency_map).map_err(|e| ContainerError::CircularDependency(e))?;

        // 按层级初始化（简化版本，顺序执行）
        let mut levels: Vec<Vec<String>> = Vec::new();

        for bean_name in sorted_beans {
            if !beans_to_init.contains(&bean_name) {
                continue;
            }

            let deps = dependency_map.get(&bean_name).cloned().unwrap_or_default();
            let level = deps
                .iter()
                .filter_map(|dep| {
                    levels
                        .iter()
                        .enumerate()
                        .find(|(_, level_beans)| level_beans.contains(dep))
                        .map(|(idx, _)| idx)
                })
                .max()
                .map(|max_level| max_level + 1)
                .unwrap_or(0);

            while levels.len() <= level {
                levels.push(Vec::new());
            }
            levels[level].push(bean_name);
        }

        // 逐层初始化（同步顺序执行）
        for level_beans in levels {
            for bean_name in level_beans {
                self.get_bean(&bean_name)?;
            }
        }

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

        let definitions = self.definitions.read();

        // 构建依赖图
        let mut dependency_map: HashMap<String, Vec<String>> = HashMap::new();

        for (name, definition) in definitions.iter() {
            dependency_map.insert(name.clone(), definition.dependencies.clone());
        }

        // 验证依赖图
        validate_dependency_graph(&dependency_map)
            .map_err(|e| ContainerError::DependencyValidationFailed(e.to_string()))?;

        tracing::info!(
            "Dependency validation passed for {} bean(s)",
            dependency_map.len()
        );

        Ok(())
    }

    /// 创建 Bean 实例并调用生命周期回调
    fn create_bean(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        let definitions = self.definitions.read();

        let definition = definitions
            .get(name)
            .ok_or_else(|| ContainerError::BeanNotFound(name.to_string()))?;

        // 使用工厂创建实例
        let instance = definition.factory.create().map_err(|e| {
            // 保留循环依赖错误，不要包装它
            match e {
                ContainerError::CircularDependency(_) => e,
                _ => ContainerError::BeanCreationFailed(format!("{}: {}", name, e)),
            }
        })?;

        let mut bean = Arc::from(instance);

        // 1. 调用 BeanPostProcessor.post_process_before_initialization
        {
            let processors = self.bean_post_processors.read();
            for processor in processors.iter() {
                bean = processor.post_process_before_initialization(bean, name)?;
            }
        }

        // 2. 调用 init 回调（如果存在）
        if let Some(ref init_fn) = definition.init_callback {
            // 需要获取可变引用来调用 init
            if let Some(bean_mut) = Arc::get_mut(&mut bean) {
                init_fn(bean_mut).map_err(|e| {
                    ContainerError::BeanCreationFailed(format!("{} init failed: {}", name, e))
                })?;
            } else {
                tracing::warn!("Cannot call init on bean '{}': multiple references exist", name);
            }
        }

        // 3. 调用 BeanPostProcessor.post_process_after_initialization
        {
            let processors = self.bean_post_processors.read();
            for processor in processors.iter() {
                bean = processor.post_process_after_initialization(bean, name)?;
            }
        }

        Ok(bean)
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
        tracing::info!("Destroying beans");

        // 获取所有定义的克隆，避免长时间持锁
        let definitions_map: std::collections::HashMap<String, bool> = {
            let definitions = self.definitions.read();

            definitions
                .iter()
                .map(|(name, def)| (name.clone(), def.destroy_callback.is_some()))
                .collect()
        };

        // 移除所有单例并尝试调用 destroy
        let mut singletons = self.singletons.write();

        let beans_to_destroy: Vec<(String, Arc<dyn Any + Send + Sync>)> =
            singletons.drain().collect();

        drop(singletons); // 释放写锁

        // 尝试对每个 Bean 调用 destroy
        for (name, mut bean) in beans_to_destroy {
            if definitions_map.get(&name).copied().unwrap_or(false) {
                // 有 destroy 回调
                let definitions = self.definitions.read();

                if let Some(definition) = definitions.get(&name) {
                    if let Some(ref destroy_fn) = definition.destroy_callback {
                        // 尝试获取可变引用（只有引用计数为1时才能成功）
                        if let Some(bean_mut) = Arc::get_mut(&mut bean) {
                            destroy_fn(bean_mut).map_err(|e| {
                                tracing::warn!("Failed to destroy bean '{}': {}", name, e);
                                e
                            })?;
                            tracing::debug!("Bean '{}' destroyed successfully", name);
                        } else {
                            tracing::warn!(
                                "Cannot destroy bean '{}': still has active references",
                                name
                            );
                        }
                    }
                }
            }
        }

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
        let name = definition.name.clone();
        let type_id = definition.factory.type_id();
        let type_name = definition.factory.type_name();

        tracing::trace!(
            "Attempting to register bean: name='{}', type='{}', scope={:?}",
            name,
            type_name,
            definition.scope
        );

        // 检查是否已存在
        {
            let definitions = self.definitions.read();
            if definitions.contains_key(&name) {
                tracing::warn!("Bean '{}' already exists, registration failed", name);
                return Err(ContainerError::BeanAlreadyExists(name));
            }
        }

        // 存储定义
        {
            let mut definitions = self.definitions.write();
            definitions.insert(name.clone(), definition);
        }

        // 注册类型到名称的映射
        {
            let mut type_to_name = self.type_to_name.write();
            type_to_name.insert(type_id, name.clone());
        }

        tracing::debug!("Bean definition registered successfully: '{}'", name);
        Ok(())
    }

    fn get_bean(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        tracing::trace!("Requesting bean: '{}'", name);

        // 先检查定义是否存在
        let scope = {
            let definitions = self.definitions.read();

            let definition = definitions.get(name).ok_or_else(|| {
                tracing::debug!("Bean '{}' not found in container", name);
                ContainerError::BeanNotFound(name.to_string())
            })?;

            definition.scope
        };

        match scope {
            Scope::Singleton => {
                // 检查缓存
                {
                    let singletons = self.singletons.read();

                    if let Some(bean) = singletons.get(name) {
                        tracing::debug!("Returning cached instance of singleton bean '{}'", name);
                        return Ok(Arc::clone(bean));
                    }
                }

                // 检查循环依赖
                if self
                    .creation_tracker
                    .is_creating(name)
                    .map_err(|e| ContainerError::Other(anyhow::anyhow!("{}", e)))?
                {
                    let creating_chain = self
                        .creation_tracker
                        .current_creating()
                        .map_err(|e| ContainerError::Other(anyhow::anyhow!("{}", e)))?;

                    tracing::error!(
                        "Circular dependency detected while creating '{}'. Creation chain: {:?}",
                        name,
                        creating_chain
                    );

                    return Err(ContainerError::CircularDependency(format!(
                        "{} -> {}",
                        creating_chain.join(" -> "),
                        name
                    )));
                }

                tracing::info!("Creating shared instance of singleton bean '{}'", name);

                // 标记为正在创建
                if !self
                    .creation_tracker
                    .start_creating(name)
                    .map_err(|e| ContainerError::Other(anyhow::anyhow!("{}", e)))?
                {
                    return Err(ContainerError::CircularDependency(format!(
                        "Detected circular dependency on '{}'",
                        name
                    )));
                }

                // 使用 RAII 模式确保在任何情况下都会清理标记
                struct CreationGuard<'a> {
                    tracker: &'a CreationTracker,
                    name: String,
                }

                impl<'a> Drop for CreationGuard<'a> {
                    fn drop(&mut self) {
                        if let Err(e) = self.tracker.finish_creating(&self.name) {
                            tracing::error!(
                                "Failed to clear creation tracker for '{}': {}",
                                self.name,
                                e
                            );
                        }
                    }
                }

                let _guard = CreationGuard {
                    tracker: &self.creation_tracker,
                    name: name.to_string(),
                };

                // 创建新实例
                let bean = self.create_bean(name)?;

                // 缓存实例
                let mut singletons = self.singletons.write();
                singletons.insert(name.to_string(), Arc::clone(&bean));

                tracing::debug!("Singleton bean '{}' created and cached", name);
                Ok(bean)
            }
            Scope::Prototype => {
                tracing::debug!("Creating new instance of prototype bean '{}'", name);
                // 每次创建新实例
                self.create_bean(name)
            }
        }
    }

    fn get_bean_by_type<T: Any + Send + Sync>(&self) -> ContainerResult<Arc<T>> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // 首先尝试通过 TypeId 查找
        let name_opt = {
            let type_to_name = self.type_to_name.read();
            type_to_name.get(&type_id).cloned()
        };

        if let Some(name) = name_opt {
            let bean = self.get_bean(&name)?;
            bean.downcast::<T>()
                .map_err(|_| ContainerError::TypeMismatch {
                    expected: type_name.to_string(),
                    found: "unknown".to_string(),
                })
        } else {
            // TypeId查找失败，尝试类型名称匹配
            let name_opt = {
                let definitions = self.definitions.read();

                let mut found_name = None;
                for (name, definition) in definitions.iter() {
                    if definition.factory.type_name() == type_name {
                        found_name = Some(name.clone());
                        break;
                    }
                }
                found_name
            };

            if let Some(name) = name_opt {
                let bean = self.get_bean(&name)?;
                bean.downcast::<T>()
                    .map_err(|_| ContainerError::TypeMismatch {
                        expected: type_name.to_string(),
                        found: "unknown".to_string(),
                    })
            } else {
                Err(ContainerError::BeanNotFound(format!(
                    "No bean found for type '{}'",
                    type_name
                )))
            }
        }
    }

    fn contains_bean(&self, name: &str) -> bool {
        self.definitions.read().contains_key(name)
    }

    fn contains_bean_by_type<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // TypeId 查找
        if self.type_to_name.read().contains_key(&type_id) {
            return true;
        }

        // TypeId 查找失败，尝试类型名称
        let definitions = self.definitions.read();
        for definition in definitions.values() {
            if definition.factory.type_name() == type_name {
                return true;
            }
        }
        false
    }

    fn get_bean_names(&self) -> Vec<String> {
        self.definitions.read().keys().cloned().collect()
    }
}

impl ApplicationContext {
    /// 导出所有 bean 定义和类型映射（内部方法，供 ApplicationContextBuilder 使用）
    ///
    /// 注意：此方法会清空当前 context 的定义，应该只在转移所有权时调用
    pub(crate) fn export_definitions(&self) -> (HashMap<String, BeanDefinition>, HashMap<TypeId, String>) {
        let mut definitions = self.definitions.write();
        let mut type_to_name = self.type_to_name.write();

        // 使用 take 来获取内容，留下空的 HashMap
        let defs = std::mem::take(&mut *definitions);
        let types = std::mem::take(&mut *type_to_name);

        (defs, types)
    }

    /// 导入 bean 定义和类型映射（内部方法，供 ApplicationContextBuilder 使用）
    pub(crate) fn import_definitions(&self, definitions: HashMap<String, BeanDefinition>, type_to_name: HashMap<TypeId, String>) {
        let mut defs = self.definitions.write();
        let mut types = self.type_to_name.write();
        *defs = definitions;
        *types = type_to_name;
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

    /// 注册框架核心组件（内部方法，不可扩展）
    fn register_core_components(context: &Arc<ApplicationContext>) -> ContainerResult<()> {
        tracing::debug!("Registering framework core components...");

        // 1. 注册 ApplicationContext 自身
        Self::register_application_context(context)?;

        // 2. 注册 Environment
        Self::register_environment(context)?;

        // 3. 注册 EventPublisher
        Self::register_event_publisher(context)?;

        tracing::info!("Framework core components registered successfully");
        Ok(())
    }

    /// 注册 ApplicationContext 自身到容器
    ///
    /// Bean名称: "applicationContext"
    /// 类型: Arc<ApplicationContext>
    fn register_application_context(
        context: &Arc<ApplicationContext>,
    ) -> ContainerResult<()> {
        tracing::trace!("Registering ApplicationContext as bean");

        let context_clone = Arc::clone(context);
        let definition = BeanDefinition::new(
            constants::APPLICATION_CONTEXT_BEAN_NAME,
            FunctionFactory::<Arc<ApplicationContext>, _>::new(move || {
                Ok(Arc::clone(&context_clone))
            }),
        )
        .with_scope(Scope::Singleton);

        context.register(definition)?;

        tracing::debug!("ApplicationContext registered as bean 'applicationContext'");
        Ok(())
    }

    /// 注册 Environment 到容器
    ///
    /// Bean名称: "environment"
    /// 类型: Arc<Environment>
    fn register_environment(context: &Arc<ApplicationContext>) -> ContainerResult<()> {
        tracing::trace!("Registering Environment as bean");

        let env = Arc::clone(context.environment());
        let definition = BeanDefinition::new(
            constants::ENVIRONMENT_BEAN_NAME,
            FunctionFactory::<Arc<crate::Environment>, _>::new(move || {
                Ok(Arc::clone(&env))
            }),
        )
        .with_scope(Scope::Singleton);

        context.register(definition)?;

        tracing::debug!("Environment registered as bean 'environment'");
        Ok(())
    }

    /// 注册 EventPublisher 到容器
    ///
    /// Bean名称: "eventPublisher"
    /// 类型: Arc<ApplicationEventPublisher>
    fn register_event_publisher(context: &Arc<ApplicationContext>) -> ContainerResult<()> {
        tracing::trace!("Registering EventPublisher as bean");

        let publisher = Arc::clone(context.event_publisher());
        let definition = BeanDefinition::new(
            constants::EVENT_PUBLISHER_BEAN_NAME,
            FunctionFactory::<Arc<crate::ApplicationEventPublisher>, _>::new(move || {
                Ok(Arc::clone(&publisher))
            }),
        )
        .with_scope(Scope::Singleton);

        context.register(definition)?;

        tracing::debug!("EventPublisher registered as bean 'eventPublisher'");
        Ok(())
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

        // 自动注册框架核心组件
        tracing::debug!("Auto-registering framework core components...");
        Self::register_core_components(&context)
            .map_err(|e| {
                tracing::error!("Failed to register core components: {}", e);
                e
            })?;
        tracing::info!("Core components auto-registration completed");

        // 注意：不在这里初始化，等待所有组件扫描完成后再初始化
        // 这样可以确保 @Component 和 @ConfigurationProperties 都被注册后才初始化
        Ok(context)
    }
}

impl Default for ApplicationContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// ApplicationContext 实现 CoreComponent
impl CoreComponent for ApplicationContext {
    fn core_bean_name() -> &'static str {
        crate::constants::APPLICATION_CONTEXT_BEAN_NAME
    }

    fn get_from_context(context: &Arc<ApplicationContext>) -> Arc<Self> {
        Arc::clone(context)
    }
}

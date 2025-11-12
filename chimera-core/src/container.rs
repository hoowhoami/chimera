use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::constants;
use crate::{
    bean::{BeanDefinition, FunctionFactory},
    config::Environment,
    error::{ContainerError, ContainerResult},
    event::{ApplicationShutdownEvent, AsyncEventPublisher, Event, EventListener, EventPublisher},
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

/// 容器 trait - 定义依赖注入容器的核心接口
#[async_trait::async_trait]
pub trait Container: Send + Sync {
    /// 注册 Bean 定义
    fn register(&self, definition: BeanDefinition) -> ContainerResult<()>;

    /// 通过名称获取 Bean
    async fn get_bean(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>>;

    /// 通过类型获取 Bean
    async fn get_bean_by_type<T: Any + Send + Sync>(&self) -> ContainerResult<Arc<T>>;

    /// 检查是否包含指定名称的 Bean
    fn contains_bean(&self, name: &str) -> bool;

    /// 检查是否包含指定类型的 Bean
    fn contains_bean_by_type<T: Any + Send + Sync>(&self) -> bool;

    /// 获取所有 Bean 的名称
    fn get_bean_names(&self) -> Vec<String>;
}

/// 应用上下文 - Container 的默认实现
pub struct ApplicationContext {
    /// Bean 定义存储
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
    event_publisher: Arc<AsyncEventPublisher>,

    /// Shutdown hooks
    shutdown_hooks: RwLock<Vec<ShutdownHook>>,

    /// 应用名称（用于事件）
    app_name: RwLock<Option<String>>,
}

impl ApplicationContext {
    /// 创建新的应用上下文
    pub fn new() -> Self {
        Self {
            definitions: RwLock::new(HashMap::new()),
            singletons: RwLock::new(HashMap::new()),
            type_to_name: RwLock::new(HashMap::new()),
            creation_tracker: CreationTracker::new(),
            environment: Arc::new(Environment::new()),
            event_publisher: Arc::new(AsyncEventPublisher::new()),
            shutdown_hooks: RwLock::new(Vec::new()),
            app_name: RwLock::new(None),
        }
    }

    /// 设置应用名称
    pub async fn set_app_name(&self, name: String) {
        let mut app_name = self.app_name.write().await;
        *app_name = Some(name);
    }

    /// 获取应用名称
    pub async fn get_app_name(&self) -> Option<String> {
        self.app_name.read().await.clone()
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
    pub fn event_publisher(&self) -> &Arc<AsyncEventPublisher> {
        &self.event_publisher
    }

    /// 获取 EventPublisher（别名，方便使用）
    pub fn get_event_publisher(&self) -> &Arc<AsyncEventPublisher> {
        &self.event_publisher
    }

    /// 发布事件
    pub async fn publish_event(&self, event: Arc<dyn Event>) {
        self.event_publisher.publish_event(event).await;
    }

    /// 注册事件监听器
    pub async fn register_listener(&self, listener: Arc<dyn EventListener>) {
        self.event_publisher.register_listener(listener).await;
    }

    /// 注册 shutdown hook
    ///
    /// Shutdown hook 会在应用关闭时按注册顺序执行
    pub async fn register_shutdown_hook<F>(&self, hook: F)
    where
        F: Fn() -> ContainerResult<()> + Send + Sync + 'static,
    {
        let mut hooks = self.shutdown_hooks.write().await;
        hooks.push(Box::new(hook));
        tracing::debug!("Registered shutdown hook, total: {}", hooks.len());
    }

    /// 构建器模式创建上下文
    pub fn builder() -> ApplicationContextBuilder {
        ApplicationContextBuilder::new()
    }

    /// 注册 Bean
    pub fn register_bean<T, F, Fut>(
        &self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<()>
    where
        T: Any + Send + Sync,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ContainerResult<T>> + Send + 'static,
    {
        let name = name.into();
        let definition = BeanDefinition::new(name.clone(), FunctionFactory::new(factory));
        self.register(definition)
    }

    /// 注册单例 Bean
    pub fn register_singleton<T, F, Fut>(
        &self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<()>
    where
        T: Any + Send + Sync,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ContainerResult<T>> + Send + 'static,
    {
        let name = name.into();
        let definition = BeanDefinition::new(name.clone(), FunctionFactory::new(factory))
            .with_scope(Scope::Singleton);
        self.register(definition)
    }

    /// 注册原型 Bean
    pub fn register_prototype<T, F, Fut>(
        &self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<()>
    where
        T: Any + Send + Sync,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ContainerResult<T>> + Send + 'static,
    {
        let name = name.into();
        let definition = BeanDefinition::new(name.clone(), FunctionFactory::new(factory))
            .with_scope(Scope::Prototype);
        self.register(definition)
    }

    /// 初始化所有非延迟加载的单例 Bean
    pub async fn initialize(&self) -> ContainerResult<()> {
        use crate::utils::dependency::topological_sort;

        // 获取所有需要初始化的 bean 及其依赖关系
        let (beans_to_init, dependency_map) = {
            let definitions = self.definitions.read().await;

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

        // 按层级并发初始化
        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut _initialized: std::collections::HashSet<String> = std::collections::HashSet::new();

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

        // 逐层并发初始化
        for level_beans in levels {
            let tasks: Vec<_> = level_beans
                .into_iter()
                .map(|bean_name| {
                    let self_ref = self;
                    async move { self_ref.get_bean(&bean_name).await }
                })
                .collect();

            // 并发执行当前层级的所有 bean 初始化
            let results = futures::future::join_all(tasks).await;

            // 检查是否有错误
            for result in results {
                result?;
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
    pub async fn validate_dependencies(&self) -> ContainerResult<()> {
        use crate::utils::dependency::validate_dependency_graph;

        let definitions = self.definitions.read().await;

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
    async fn create_bean(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        let definitions = self.definitions.read().await;

        let definition = definitions
            .get(name)
            .ok_or_else(|| ContainerError::BeanNotFound(name.to_string()))?;

        // 使用工厂创建实例
        let mut instance = definition.factory.create().await.map_err(|e| {
            // 保留循环依赖错误，不要包装它
            match e {
                ContainerError::CircularDependency(_) => e,
                _ => ContainerError::BeanCreationFailed(format!("{}: {}", name, e)),
            }
        })?;

        // 调用 init 回调（如果存在）
        if let Some(ref init_fn) = definition.init_callback {
            init_fn(instance.as_mut()).map_err(|e| {
                ContainerError::BeanCreationFailed(format!("{} init failed: {}", name, e))
            })?;
        }

        Ok(Arc::from(instance))
    }

    /// 销毁所有单例 Bean（调用 destroy 回调）
    /// 注意：只有当 Arc 的引用计数为 1 时才能调用 destroy
    pub async fn shutdown(&self) -> ContainerResult<()> {
        tracing::info!("Starting application shutdown");

        // 1. 发布 ApplicationShutdownEvent
        let app_name = self
            .get_app_name()
            .await
            .unwrap_or_else(|| "Application".to_string());
        let shutdown_event = Arc::new(ApplicationShutdownEvent::new(app_name));
        self.publish_event(shutdown_event).await;

        // 2. 执行所有 shutdown hooks
        let hooks = self.shutdown_hooks.read().await;
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
            let definitions = self.definitions.read().await;

            definitions
                .iter()
                .map(|(name, def)| (name.clone(), def.destroy_callback.is_some()))
                .collect()
        };

        // 移除所有单例并尝试调用 destroy
        let mut singletons = self.singletons.write().await;

        let beans_to_destroy: Vec<(String, Arc<dyn Any + Send + Sync>)> =
            singletons.drain().collect();

        drop(singletons); // 释放写锁

        // 尝试对每个 Bean 调用 destroy
        for (name, mut bean) in beans_to_destroy {
            if definitions_map.get(&name).copied().unwrap_or(false) {
                // 有 destroy 回调
                let definitions = self.definitions.read().await;

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

#[async_trait::async_trait]
impl Container for ApplicationContext {
    fn register(&self, definition: BeanDefinition) -> ContainerResult<()> {
        // 注册方法保持同步（不涉及bean创建）
        let name = definition.name.clone();
        let type_id = definition.factory.type_id();
        let type_name = definition.factory.type_name();

        tracing::trace!(
            "Attempting to register bean: name='{}', type='{}', scope={:?}",
            name,
            type_name,
            definition.scope
        );

        // 使用 blocking 方式访问（仅用于注册，不在异步上下文中）
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                // 检查是否已存在
                {
                    let definitions = self.definitions.read().await;
                    if definitions.contains_key(&name) {
                        tracing::warn!("Bean '{}' already exists, registration failed", name);
                        return Err(ContainerError::BeanAlreadyExists(name));
                    }
                }

                // 存储定义
                {
                    let mut definitions = self.definitions.write().await;
                    definitions.insert(name.clone(), definition);
                }

                // 注册类型到名称的映射
                {
                    let mut type_to_name = self.type_to_name.write().await;
                    type_to_name.insert(type_id, name.clone());
                }

                tracing::debug!("Bean definition registered successfully: '{}'", name);
                Ok(())
            })
        })
    }

    async fn get_bean(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        tracing::trace!("Requesting bean: '{}'", name);

        // 先检查定义是否存在
        let scope = {
            let definitions = self.definitions.read().await;

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
                    let singletons = self.singletons.read().await;

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
                let bean = self.create_bean(name).await?;

                // 缓存实例
                let mut singletons = self.singletons.write().await;
                singletons.insert(name.to_string(), Arc::clone(&bean));

                tracing::debug!("Singleton bean '{}' created and cached", name);
                Ok(bean)
            }
            Scope::Prototype => {
                tracing::debug!("Creating new instance of prototype bean '{}'", name);
                // 每次创建新实例
                self.create_bean(name).await
            }
        }
    }

    async fn get_bean_by_type<T: Any + Send + Sync>(&self) -> ContainerResult<Arc<T>> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // 首先尝试通过 TypeId 查找
        let name_opt = {
            let type_to_name = self.type_to_name.read().await;
            type_to_name.get(&type_id).cloned()
        };

        if let Some(name) = name_opt {
            let bean = self.get_bean(&name).await?;
            bean.downcast::<T>()
                .map_err(|_| ContainerError::TypeMismatch {
                    expected: type_name.to_string(),
                    found: "unknown".to_string(),
                })
        } else {
            // TypeId查找失败，尝试类型名称匹配
            let name_opt = {
                let definitions = self.definitions.read().await;

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
                let bean = self.get_bean(&name).await?;
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
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async { self.definitions.read().await.contains_key(name) })
        })
    }

    fn contains_bean_by_type<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                // TypeId查找
                if self.type_to_name.read().await.contains_key(&type_id) {
                    return true;
                }

                // TypeId查找失败，尝试类型名称
                let definitions = self.definitions.read().await;
                for definition in definitions.values() {
                    if definition.factory.type_name() == type_name {
                        return true;
                    }
                }
                false
            })
        })
    }

    fn get_bean_names(&self) -> Vec<String> {
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async { self.definitions.read().await.keys().cloned().collect() })
        })
    }
}

/// 应用上下文构建器
pub struct ApplicationContextBuilder {
    context: ApplicationContext,
}

impl ApplicationContextBuilder {
    pub fn new() -> Self {
        Self {
            context: ApplicationContext::new(),
        }
    }

    /// 注册框架核心组件（内部方法，不可扩展）
    async fn register_core_components(context: &Arc<ApplicationContext>) -> ContainerResult<()> {
        tracing::debug!("Registering framework core components...");

        // 1. 注册 ApplicationContext 自身
        Self::register_application_context(context).await?;

        // 2. 注册 Environment
        Self::register_environment(context).await?;

        // 3. 注册 EventPublisher
        Self::register_event_publisher(context).await?;

        tracing::info!("Framework core components registered successfully");
        Ok(())
    }

    /// 注册 ApplicationContext 自身到容器
    ///
    /// Bean名称: "applicationContext"
    /// 类型: Arc<ApplicationContext>
    async fn register_application_context(
        context: &Arc<ApplicationContext>,
    ) -> ContainerResult<()> {
        tracing::trace!("Registering ApplicationContext as bean");

        let context_clone = Arc::clone(context);
        let definition = BeanDefinition::new(
            constants::APPLICATION_CONTEXT_BEAN_NAME,
            FunctionFactory::<Arc<ApplicationContext>, _, _>::new(move || {
                let ctx = Arc::clone(&context_clone);
                async move { Ok(ctx) }
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
    async fn register_environment(context: &Arc<ApplicationContext>) -> ContainerResult<()> {
        tracing::trace!("Registering Environment as bean");

        let env = Arc::clone(context.environment());
        let definition = BeanDefinition::new(
            constants::ENVIRONMENT_BEAN_NAME,
            FunctionFactory::<Arc<crate::Environment>, _, _>::new(move || {
                let environment = Arc::clone(&env);
                async move { Ok(environment) }
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
    /// 类型: Arc<AsyncEventPublisher>
    async fn register_event_publisher(context: &Arc<ApplicationContext>) -> ContainerResult<()> {
        tracing::trace!("Registering EventPublisher as bean");

        let publisher = Arc::clone(context.event_publisher());
        let definition = BeanDefinition::new(
            constants::EVENT_PUBLISHER_BEAN_NAME,
            FunctionFactory::<Arc<crate::AsyncEventPublisher>, _, _>::new(move || {
                let event_publisher = Arc::clone(&publisher);
                async move { Ok(event_publisher) }
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
    pub fn register_singleton<T, F, Fut>(
        self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<Self>
    where
        T: Any + Send + Sync,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ContainerResult<T>> + Send + 'static,
    {
        self.context.register_singleton(name, factory)?;
        Ok(self)
    }

    /// 注册原型 Bean
    pub fn register_prototype<T, F, Fut>(
        self,
        name: impl Into<String>,
        factory: F,
    ) -> ContainerResult<Self>
    where
        T: Any + Send + Sync,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ContainerResult<T>> + Send + 'static,
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
    pub async fn build(self) -> ContainerResult<Arc<ApplicationContext>> {
        // 构建上下文
        let context = Arc::new(self.context);

        // 自动注册框架核心组件
        tracing::debug!("Auto-registering framework core components...");
        Self::register_core_components(&context)
            .await
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

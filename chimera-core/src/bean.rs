use std::any::{Any, TypeId};
use std::fmt;
use crate::{ContainerResult, Scope};

/// Bean trait - 所有可以被容器管理的类型都需要实现此 trait
pub trait Bean: Any + Send + Sync {
    /// 获取 Bean 的名称
    fn bean_name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// 为所有满足条件的类型自动实现 Bean trait
impl<T: Any + Send + Sync> Bean for T {}

/// Bean 工厂 trait - 用于创建 Bean 实例
#[async_trait::async_trait]
pub trait BeanFactory: Send + Sync {
    /// 创建 Bean 实例
    async fn create(&self) -> ContainerResult<Box<dyn Any + Send + Sync>>;

    /// 获取 Bean 的类型 ID
    fn type_id(&self) -> TypeId;

    /// 获取 Bean 的类型名称
    fn type_name(&self) -> &str;
}

/// 生命周期回调类型
pub type InitCallback = Box<dyn Fn(&mut dyn Any) -> ContainerResult<()> + Send + Sync>;
pub type DestroyCallback = Box<dyn Fn(&mut dyn Any) -> ContainerResult<()> + Send + Sync>;

/// Bean 定义 - 描述如何创建和管理 Bean
pub struct BeanDefinition {
    /// Bean 的名称
    pub name: String,

    /// Bean 的作用域
    pub scope: Scope,

    /// Bean 工厂
    pub factory: Box<dyn BeanFactory>,

    /// 是否延迟初始化（仅对单例有效）
    pub lazy: bool,

    /// Bean 的依赖列表（用于静态依赖分析）
    pub dependencies: Vec<String>,

    /// 初始化回调（@PostConstruct）
    pub init_callback: Option<InitCallback>,

    /// 销毁回调（@PreDestroy）
    pub destroy_callback: Option<DestroyCallback>,
}

impl BeanDefinition {
    /// 创建新的 Bean 定义
    pub fn new<F>(name: impl Into<String>, factory: F) -> Self
    where
        F: BeanFactory + 'static,
    {
        Self {
            name: name.into(),
            scope: Scope::default(),
            factory: Box::new(factory),
            lazy: false,
            dependencies: Vec::new(),
            init_callback: None,
            destroy_callback: None,
        }
    }

    /// 设置作用域
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// 设置延迟初始化
    pub fn with_lazy(mut self, lazy: bool) -> Self {
        self.lazy = lazy;
        self
    }

    /// 设置依赖列表
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// 设置初始化回调
    pub fn with_init<F>(mut self, init_fn: F) -> Self
    where
        F: Fn(&mut dyn Any) -> ContainerResult<()> + Send + Sync + 'static,
    {
        self.init_callback = Some(Box::new(init_fn));
        self
    }

    /// 设置销毁回调
    pub fn with_destroy<F>(mut self, destroy_fn: F) -> Self
    where
        F: Fn(&mut dyn Any) -> ContainerResult<()> + Send + Sync + 'static,
    {
        self.destroy_callback = Some(Box::new(destroy_fn));
        self
    }
}

impl fmt::Debug for BeanDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BeanDefinition")
            .field("name", &self.name)
            .field("scope", &self.scope)
            .field("lazy", &self.lazy)
            .field("dependencies", &self.dependencies)
            .field("type_name", &self.factory.type_name())
            .finish()
    }
}

/// 简单的函数工厂实现
pub struct FunctionFactory<T, F, Fut>
where
    T: Any + Send + Sync,
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ContainerResult<T>> + Send,
{
    factory_fn: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F, Fut> FunctionFactory<T, F, Fut>
where
    T: Any + Send + Sync,
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ContainerResult<T>> + Send,
{
    pub fn new(factory_fn: F) -> Self {
        Self {
            factory_fn,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T, F, Fut> BeanFactory for FunctionFactory<T, F, Fut>
where
    T: Any + Send + Sync,
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ContainerResult<T>> + Send,
{
    async fn create(&self) -> ContainerResult<Box<dyn Any + Send + Sync>> {
        let instance = (self.factory_fn)().await?;
        Ok(Box::new(instance))
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn type_name(&self) -> &str {
        std::any::type_name::<T>()
    }
}

use crate::{ApplicationContext, ContainerResult, Scope, Container};
use crate::event::EventListener;
use std::sync::Arc;

/// Component注册函数类型
pub type ComponentRegistrar = fn(&Arc<ApplicationContext>) -> ContainerResult<()>;

/// Component注册表 - 用于inventory收集
pub struct ComponentRegistry {
    pub registrar: ComponentRegistrar,
    pub name: &'static str,
}

inventory::collect!(ComponentRegistry);

/// ConfigurationProperties注册函数类型
pub type ConfigurationPropertiesRegistrar = fn(&Arc<ApplicationContext>) -> ContainerResult<()>;

/// ConfigurationProperties注册表 - 用于inventory收集
pub struct ConfigurationPropertiesRegistry {
    pub registrar: ConfigurationPropertiesRegistrar,
    pub name: &'static str,
}

inventory::collect!(ConfigurationPropertiesRegistry);

/// EventListener注册函数类型
pub type EventListenerRegistrar = fn(&Arc<ApplicationContext>) -> ContainerResult<Arc<dyn EventListener>>;

/// EventListener注册表 - 用于inventory收集
pub struct EventListenerRegistry {
    pub registrar: EventListenerRegistrar,
    pub name: &'static str,
}

inventory::collect!(EventListenerRegistry);

/// Component trait - 用于标记可以自动注册到容器的组件
///
/// 通过 #[derive(Component)] 宏自动实现
///
/// # 示例
///
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_core_macros::Component;
/// use std::sync::Arc;
///
/// #[derive(Component)]
/// #[bean("userService")]
/// #[scope("singleton")]
/// struct UserService {
///     #[autowired]
///     db: Arc<DatabaseService>,
/// }
/// ```
pub trait Component: Sized + Send + Sync + 'static {
    /// 获取 Bean 名称
    fn bean_name() -> &'static str;

    /// 获取作用域
    fn scope() -> Scope {
        Scope::Singleton
    }

    /// 是否延迟初始化
    fn lazy() -> bool {
        false
    }

    /// 获取依赖的 bean 名称列表
    fn dependencies() -> Vec<String> {
        Vec::new()
    }

    /// 初始化回调（类似 @PostConstruct）
    ///
    /// 返回 None 表示没有初始化逻辑
    fn init_callback() -> Option<fn(&mut Self) -> ContainerResult<()>> {
        None
    }

    /// 销毁回调（类似 @PreDestroy）
    ///
    /// 返回 None 表示没有清理逻辑
    fn destroy_callback() -> Option<fn(&mut Self) -> ContainerResult<()>> {
        None
    }

    /// 是否实现了EventListener
    ///
    /// 如果返回true，会在Bean创建后自动注册为EventListener
    fn is_event_listener() -> bool {
        false
    }

    /// 转换为EventListener（如果实现了EventListener trait）
    fn as_event_listener(self: Arc<Self>) -> Option<Arc<dyn EventListener>> {
        None
    }

    /// 从容器创建实例
    /// 自动注入依赖
    fn create_from_context(context: &Arc<ApplicationContext>) -> ContainerResult<Self>;

    /// 注册到容器
    fn register(context: &Arc<ApplicationContext>) -> ContainerResult<()> {
        let ctx = Arc::clone(context);
        let scope = Self::scope();
        let lazy = Self::lazy();
        let dependencies = Self::dependencies();

        let mut definition = crate::BeanDefinition::new(
            Self::bean_name(),
            crate::bean::FunctionFactory::new(move || {
                Self::create_from_context(&ctx)
            }),
        )
        .with_scope(scope)
        .with_lazy(lazy)
        .with_dependencies(dependencies);

        // 添加初始化回调
        if let Some(init_fn) = Self::init_callback() {
            definition = definition.with_init(move |bean: &mut dyn std::any::Any| {
                if let Some(typed_bean) = bean.downcast_mut::<Self>() {
                    init_fn(typed_bean)
                } else {
                    Err(crate::ContainerError::BeanCreationFailed(
                        "Failed to downcast bean in init callback".to_string()
                    ))
                }
            });
        }

        // 添加销毁回调
        if let Some(destroy_fn) = Self::destroy_callback() {
            definition = definition.with_destroy(move |bean: &mut dyn std::any::Any| {
                if let Some(typed_bean) = bean.downcast_mut::<Self>() {
                    destroy_fn(typed_bean)
                } else {
                    Err(crate::ContainerError::BeanCreationFailed(
                        "Failed to downcast bean in destroy callback".to_string()
                    ))
                }
            });
        }

        context.as_ref().register(definition)?;
        Ok(())
    }
}

impl ApplicationContext {
    /// 自动扫描并注册所有Component
    ///
    /// 这会自动注册所有使用#[derive(Component)]标记的类型
    pub fn scan_components(self: &Arc<Self>) -> ContainerResult<()> {
        tracing::info!("Starting component scan for @Component annotated beans");

        let components: Vec<_> = inventory::iter::<ComponentRegistry>().collect();
        let total = components.len();

        if total == 0 {
            tracing::warn!("No @Component annotated beans found in classpath");
            return Ok(());
        }

        tracing::info!("Found {} @Component annotated bean(s) to register", total);

        for (idx, component) in components.iter().enumerate() {
            tracing::debug!(
                "Registering component [{}/{}]: '{}'",
                idx + 1,
                total,
                component.name
            );

            (component.registrar)(self).map_err(|e| {
                tracing::error!("Failed to register component '{}': {}", component.name, e);
                e
            })?;
        }

        tracing::info!("Component scan completed successfully, registered {} bean(s)", total);
        Ok(())
    }

    /// 自动扫描并注册所有ConfigurationProperties
    ///
    /// 这会自动绑定所有使用#[derive(ConfigurationProperties)]标记的类型
    pub fn scan_configuration_properties(self: &Arc<Self>) -> ContainerResult<()> {
        tracing::info!("Starting configuration properties scan for @ConfigurationProperties annotated beans");

        let config_props: Vec<_> = inventory::iter::<ConfigurationPropertiesRegistry>().collect();
        let total = config_props.len();

        if total == 0 {
            tracing::debug!("No @ConfigurationProperties annotated beans found in classpath");
            return Ok(());
        }

        tracing::info!("Found {} @ConfigurationProperties annotated bean(s) to bind", total);

        for (idx, config_prop) in config_props.iter().enumerate() {
            tracing::debug!(
                "Binding configuration properties [{}/{}]: '{}'",
                idx + 1,
                total,
                config_prop.name
            );

            (config_prop.registrar)(self).map_err(|e| {
                tracing::error!("Failed to bind configuration properties '{}': {}", config_prop.name, e);
                e
            })?;
        }

        tracing::info!("Configuration properties scan completed successfully, bound {} bean(s)", total);
        Ok(())
    }

    /// 自动扫描并注册EventListener
    ///
    /// 在Bean初始化后调用，自动注册所有实现了EventListener的Component
    pub fn scan_event_listeners(self: &Arc<Self>) -> ContainerResult<()> {
        tracing::info!("Starting event listener scan for @Component beans implementing EventListener");

        let listeners: Vec<_> = inventory::iter::<EventListenerRegistry>().collect();
        let total = listeners.len();

        if total == 0 {
            tracing::debug!("No EventListener implementations found");
            return Ok(());
        }

        tracing::info!("Found {} EventListener implementation(s) to register", total);

        for (idx, listener_reg) in listeners.iter().enumerate() {
            tracing::debug!(
                "Registering event listener [{}/{}]: '{}'",
                idx + 1,
                total,
                listener_reg.name
            );

            let listener = (listener_reg.registrar)(self).map_err(|e| {
                tracing::error!("Failed to create event listener '{}': {}", listener_reg.name, e);
                e
            })?;

            self.register_listener(listener);
        }

        tracing::info!("Event listener scan completed, registered {} listener(s)", total);
        Ok(())
    }
}

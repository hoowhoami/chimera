mod attribute_helpers;
mod value_injection;
mod config_properties_impl;
mod component_impl;
mod component_attr;
mod bean_impl;
mod bean_post_processor_impl;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

/// Component派生宏
///
/// 用法：
/// ```ignore
/// #[derive(Component)]
/// #[bean("userService")]  // 可选：指定bean名称
/// #[scope("singleton")]   // 可选：指定作用域 (singleton/prototype)
/// #[lazy]                 // 可选：延迟初始化
/// #[init]                 // 可选：初始化回调（默认调用 post_construct 方法）
/// #[init("custom_init")]  // 可选：自定义初始化方法名
/// #[destroy]              // 可选：销毁回调（默认调用 pre_destroy 方法）
/// #[destroy("cleanup")]   // 可选：自定义销毁方法名
/// #[event_listener]       // 可选：自动注册为EventListener
/// ```
#[proc_macro_derive(Component, attributes(bean, scope, lazy, autowired, value, init, destroy, event_listener))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    component_impl::derive_component_impl(input)
}

/// Bean属性宏
///
/// 用于在impl块中标记Bean工厂方法
///
/// 用法：
/// ```ignore
/// impl AppConfig {
///     #[bean]
///     pub fn database_service(&self) -> Result<DatabaseService> {
///         DatabaseService::new(&self.db_url)
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn bean(_attr: TokenStream, item: TokenStream) -> TokenStream {
    bean_impl::bean_impl(_attr, item)
}

/// ConfigurationProperties派生宏
///
/// 用于批量绑定配置到结构体
///
/// 用法：
/// ```ignore
/// #[derive(ConfigurationProperties)]
/// #[prefix("database")]
/// ```
#[proc_macro_derive(ConfigurationProperties, attributes(prefix, config))]
pub fn derive_configuration_properties(input: TokenStream) -> TokenStream {
    config_properties_impl::derive_configuration_properties_impl(input)
}

/// BeanPostProcessor派生宏
///
/// 用于自动注册 BeanPostProcessor 到框架
///
/// 注意：必须同时使用 #[derive(Component)] 才能让 BeanPostProcessor 支持依赖注入
///
/// 用法：
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_core_macros::{BeanPostProcessor, Component};
///
/// #[derive(BeanPostProcessor, Component)]
/// pub struct LoggingBeanPostProcessor {
///     #[autowired]
///     context: Arc<ApplicationContext>,
/// }
///
/// impl BeanPostProcessor for LoggingBeanPostProcessor {
///     fn post_process_after_initialization(
///         &self,
///         bean: Arc<dyn Any + Send + Sync>,
///         bean_name: &str
///     ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
///         tracing::info!("Bean initialized: {}", bean_name);
///         Ok(bean)
///     }
///
///     fn order(&self) -> i32 {
///         100  // 可选：重写 order 方法指定优先级，数字越小优先级越高
///     }
/// }
/// ```
#[proc_macro_derive(BeanPostProcessor)]
pub fn derive_bean_post_processor(input: TokenStream) -> TokenStream {
    bean_post_processor_impl::derive_bean_post_processor_impl(input)
}

/// Component 属性宏
///
/// 用于标记 Component 类型的 impl 块，自动检查方法名是否与 Component trait 的保留方法冲突
///
/// **必须用于所有 Component 类型的 impl 块**，否则可能导致方法名冲突
///
/// # Component trait 保留的方法名
///
/// - `bean_name()`, `scope()`, `lazy()`, `dependencies()`
/// - `init_callback()`, `destroy_callback()`
/// - `is_event_listener()`, `as_event_listener()`
/// - `create_from_context()`, `register()`
///
/// # 用法
///
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_core_macros::Component;
///
/// #[derive(Component)]
/// struct UserService {
///     #[autowired]
///     db: Arc<DatabaseService>,
/// }
///
/// #[component]  // 必须添加此属性
/// impl UserService {
///     pub fn create_user(&self) { }   // ✅ OK
///     pub fn user_register(&self) { } // ✅ OK
///     pub fn register(&self) { }      // ❌ 编译错误：与 Component::register 冲突
/// }
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    component_attr::component_impl(attr, item)
}

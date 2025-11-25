mod attribute_helpers;
mod value_injection;
mod config_properties_impl;
mod component_impl;
mod component_attr;
mod configuration_impl;
mod configuration_attr;
mod bean_impl;
mod bean_post_processor_impl;
mod bean_factory_post_processor_impl;
mod smart_initializing_singleton_impl;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

/// Component派生宏
///
/// 用法：
/// ```ignore
/// #[derive(Component)]
/// #[component("userService")]  // 可选：指定bean名称（简写形式）
/// // 或
/// #[component(name = "userService")]  // 可选：指定bean名称（完整形式）
/// #[scope("singleton")]   // 可选：指定作用域 (singleton/prototype)
/// #[lazy]                 // 可选：延迟初始化
/// #[init]                 // 可选：初始化回调（默认调用 init 方法）
/// #[init("custom_init")]  // 可选：自定义初始化方法名
/// #[destroy]              // 可选：销毁回调（默认调用 destroy 方法）
/// #[destroy("cleanup")]   // 可选：自定义销毁方法名
/// #[event_listener]       // 可选：自动注册为EventListener
/// ```
#[proc_macro_derive(Component, attributes(component, scope, lazy, autowired, value, init, destroy, event_listener))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    component_impl::derive_component_impl(input)
}

/// Bean属性宏
///
/// 用于标记 Configuration 类中的 Bean 工厂方法
///
/// **注意**：此宏只是一个标记，真正的注册由 `#[configuration]` 属性宏处理
/// 必须在 Configuration 的 impl 块上添加 `#[configuration]` 属性
///
/// # 用法
///
/// ```ignore
/// #[derive(Configuration)]
/// pub struct AppConfig {
///     #[autowired]
///     environment: Arc<Environment>,
/// }
///
/// #[configuration]  // 必须添加此属性
/// impl AppConfig {
///     /// 使用方法名作为 bean 名称
///     #[bean]
///     pub fn database_service(&self) -> DatabaseService {
///         DatabaseService::new()
///     }
///
///     /// 指定自定义 bean 名称
///     #[bean("customDb")]
///     pub fn db(&self) -> DatabaseService {
///         DatabaseService::new()
///     }
///
///     /// 使用 #[scope] 指定作用域
///     #[bean("cache")]
///     #[scope("prototype")]
///     pub fn cache_service(&self) -> CacheService {
///         CacheService::new()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn bean(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Bean 宏现在只是一个标记，实际处理由 configuration 属性宏完成
    item
}

/// Scope 属性宏
///
/// 用于指定 Bean 方法的作用域（singleton 或 prototype）
///
/// **注意**：此宏只是一个标记，真正的处理由 `#[configuration]` 属性宏完成
/// 必须在 Configuration 的 impl 块上添加 `#[configuration]` 属性
///
/// # 用法
///
/// ```ignore
/// #[derive(Configuration)]
/// pub struct AppConfig;
///
/// #[configuration]
/// impl AppConfig {
///     /// 单例 bean（默认）
///     #[bean]
///     pub fn singleton_service(&self) -> MyService {
///         MyService::new()
///     }
///
///     /// 原型 bean - 每次获取创建新实例
///     #[bean]
///     #[scope("prototype")]
///     pub fn prototype_service(&self) -> MyService {
///         MyService::new()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn scope(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Scope 宏现在只是一个标记，实际处理由 configuration 属性宏完成
    item
}

/// Lazy 属性宏
///
/// 用于标记 Bean 方法为延迟初始化（Lazy Initialization）
///
/// **注意**：此宏只是一个标记，真正的处理由 `#[configuration]` 属性宏完成
/// 必须在 Configuration 的 impl 块上添加 `#[configuration]` 属性
///
/// # 用法
///
/// ```ignore
/// #[derive(Configuration)]
/// pub struct AppConfig;
///
/// #[configuration]
/// impl AppConfig {
///     /// 延迟初始化的 bean - 只有在首次使用时才会创建
///     #[bean]
///     #[lazy]
///     pub fn expensive_service(&self) -> MyService {
///         MyService::new()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn lazy(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Lazy 宏现在只是一个标记，实际处理由 configuration 属性宏完成
    item
}

/// Init 属性宏
///
/// 用于指定 Bean 的初始化回调方法
///
/// **注意**：此宏只是一个标记，真正的处理由 `#[configuration]` 属性宏完成
/// 必须在 Configuration 的 impl 块上添加 `#[configuration]` 属性
///
/// # 用法
///
/// ```ignore
/// #[derive(Configuration)]
/// pub struct AppConfig;
///
/// pub struct MyService {
///     // fields...
/// }
///
/// impl MyService {
///     pub fn init(&mut self) -> ContainerResult<()> {
///         // 初始化逻辑
///         Ok(())
///     }
///
///     pub fn custom_init(&mut self) -> ContainerResult<()> {
///         // 自定义初始化逻辑
///         Ok(())
///     }
/// }
///
/// #[configuration]
/// impl AppConfig {
///     /// 使用默认的 init 方法
///     #[bean]
///     #[init]
///     pub fn my_service(&self) -> MyService {
///         MyService::new()
///     }
///
///     /// 指定自定义初始化方法
///     #[bean]
///     #[init("custom_init")]
///     pub fn another_service(&self) -> MyService {
///         MyService::new()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn init(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Init 宏现在只是一个标记，实际处理由 configuration 属性宏完成
    item
}

/// Destroy 属性宏
///
/// 用于指定 Bean 的销毁回调方法
///
/// **注意**：此宏只是一个标记，真正的处理由 `#[configuration]` 属性宏完成
/// 必须在 Configuration 的 impl 块上添加 `#[configuration]` 属性
///
/// # 用法
///
/// ```ignore
/// #[derive(Configuration)]
/// pub struct AppConfig;
///
/// pub struct MyService {
///     // fields...
/// }
///
/// impl MyService {
///     pub fn destroy(&mut self) -> ContainerResult<()> {
///         // 清理逻辑
///         Ok(())
///     }
///
///     pub fn cleanup(&mut self) -> ContainerResult<()> {
///         // 自定义清理逻辑
///         Ok(())
///     }
/// }
///
/// #[configuration]
/// impl AppConfig {
///     /// 使用默认的 destroy 方法
///     #[bean]
///     #[destroy]
///     pub fn my_service(&self) -> MyService {
///         MyService::new()
///     }
///
///     /// 指定自定义销毁方法
///     #[bean]
///     #[destroy("cleanup")]
///     pub fn another_service(&self) -> MyService {
///         MyService::new()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn destroy(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Destroy 宏现在只是一个标记，实际处理由 configuration 属性宏完成
    item
}

/// Configuration impl 块属性宏
///
/// 用于 Configuration 类的 impl 块，自动扫描和注册所有 #[bean] 方法
///
/// 类似 Spring 的 @Configuration + @Bean 组合
///
/// # 用法
///
/// ```ignore
/// #[derive(Configuration)]
/// pub struct AppConfig {
///     #[autowired]
///     environment: Arc<Environment>,
/// }
///
/// #[configuration]
/// impl AppConfig {
///     #[bean]
///     pub fn email_service(&self) -> EmailService {
///         EmailService::new()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn configuration(attr: TokenStream, item: TokenStream) -> TokenStream {
    configuration_attr::configuration_impl(attr, item)
}

/// Configuration派生宏
///
/// 用于标记配置类，配置类是特殊的 Component，用于包含 `#[bean]` 工厂方法
///
/// 类似 Spring 的 @Configuration 注解
///
/// # 用法
///
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_core_macros::Configuration;
///
/// #[derive(Configuration)]
/// pub struct AppConfig {
///     #[autowired]
///     environment: Arc<Environment>,
/// }
///
/// impl AppConfig {
///     #[bean]
///     pub fn database_service(&self) -> ContainerResult<DatabaseService> {
///         DatabaseService::new(&self.environment.get_string("db.url").unwrap())
///     }
///
///     #[bean("cacheService")]
///     #[scope("prototype")]
///     pub fn cache(&self) -> ContainerResult<CacheService> {
///         CacheService::new()
///     }
/// }
/// ```
///
/// # 特点
///
/// - Configuration 本身也是一个 Component，会被自动注册到容器
/// - 支持 `#[component("name")]` 指定配置类的 bean 名称
/// - 支持 `#[scope]`, `#[lazy]`, `#[init]`, `#[destroy]` 等 Component 属性
/// - 其中的 `#[bean]` 方法会在 `scan_bean_methods()` 时被扫描和注册
#[proc_macro_derive(Configuration, attributes(component, scope, lazy, autowired, value, init, destroy))]
pub fn derive_configuration(input: TokenStream) -> TokenStream {
    configuration_impl::derive_configuration_impl(input)
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

/// BeanFactoryPostProcessor 派生宏
///
/// 用于自动注册 BeanFactoryPostProcessor 到框架
///
/// 注意：必须同时使用 #[derive(Component)] 才能让 BeanFactoryPostProcessor 支持依赖注入
///
/// 用法：
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_core_macros::{BeanFactoryPostProcessor, Component};
///
/// #[derive(BeanFactoryPostProcessor, Component)]
/// pub struct CustomBeanFactoryPostProcessor;
///
/// impl BeanFactoryPostProcessor for CustomBeanFactoryPostProcessor {
///     fn post_process_bean_factory(&self, context: &Arc<ApplicationContext>) -> ContainerResult<()> {
///         tracing::info!("Processing bean factory");
///         Ok(())
///     }
///
///     fn order(&self) -> i32 {
///         100  // 可选：重写 order 方法指定优先级，数字越小优先级越高
///     }
/// }
/// ```
#[proc_macro_derive(BeanFactoryPostProcessor)]
pub fn derive_bean_factory_post_processor(input: TokenStream) -> TokenStream {
    bean_factory_post_processor_impl::derive_bean_factory_post_processor_impl(input)
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

/// SmartInitializingSingleton 派生宏
///
/// 自动注册实现了 SmartInitializingSingleton trait 的 Bean
///
/// 用法：
/// ```ignore
/// use chimera_core::prelude::*;
/// use chimera_core_macros::{SmartInitializingSingleton, Component};
///
/// #[derive(SmartInitializingSingleton, Component)]
/// pub struct StartupService {
///     #[autowired]
///     app_context: Arc<ApplicationContext>,
/// }
///
/// impl SmartInitializingSingleton for StartupService {
///     fn after_singletons_instantiated(&self) -> ContainerResult<()> {
///         tracing::info!("All singletons initialized!");
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_derive(SmartInitializingSingleton)]
pub fn derive_smart_initializing_singleton(input: TokenStream) -> TokenStream {
    smart_initializing_singleton_impl::derive_smart_initializing_singleton_impl(input)
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

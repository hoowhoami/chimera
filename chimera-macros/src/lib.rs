mod attribute_helpers;
mod value_injection;
mod config_properties_impl;
mod component_impl;
mod bean_impl;

use proc_macro::TokenStream;

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
/// ```
#[proc_macro_derive(Component, attributes(bean, scope, lazy, autowired, value, init, destroy))]
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

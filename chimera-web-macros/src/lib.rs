//! Chimera Web Macros
//!
//! 提供 Web 相关的过程宏，类似 Spring MVC 的注解

mod controller;
mod route;
mod utils;

use proc_macro::TokenStream;

/// Controller 宏
///
/// 将结构体标记为控制器，自动注册路由
///
/// # 示例
///
/// ```ignore
/// #[derive(Controller)]
/// #[request_mapping("/api/users")]
/// struct UserController {
///     #[autowired]
///     service: Arc<UserService>,
/// }
/// ```
#[proc_macro_derive(Controller, attributes(request_mapping))]
pub fn derive_controller(input: TokenStream) -> TokenStream {
    controller::derive_controller_impl(input)
}

/// 处理控制器实现块，提取路由方法
///
/// # 示例
///
/// ```ignore
/// #[controller_impl]
/// impl UserController {
///     #[get_mapping("/:id")]
///     async fn get_user(&self, #[path_param] id: String) -> RestResponse<User> {
///         // ...
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn controller_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::controller_impl(attr, item)
}

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
/// #[route("/api/users")]
/// struct UserController {
///     #[autowired]
///     service: Arc<UserService>,
/// }
/// ```
#[proc_macro_derive(Controller, attributes(route))]
pub fn derive_controller(input: TokenStream) -> TokenStream {
    controller::derive_controller_impl(input)
}

/// 处理控制器实现块，提取路由方法
///
/// # 示例
///
/// ```ignore
/// #[controller]
/// impl UserController {
///     #[get_mapping("/:id")]
///     async fn get_user(&self, id: String) -> ResponseEntity<User> {
///         // ...
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::controller_impl(attr, item)
}

/// GET 路由映射
///
/// # 示例
///
/// ```ignore
/// #[get_mapping("/info")]
/// async fn get_user(&self) -> impl IntoResponse {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn get_mapping(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::route_mapping_impl("get", attr, item)
}

/// POST 路由映射
#[proc_macro_attribute]
pub fn post_mapping(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::route_mapping_impl("post", attr, item)
}

/// PUT 路由映射
#[proc_macro_attribute]
pub fn put_mapping(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::route_mapping_impl("put", attr, item)
}

/// DELETE 路由映射
#[proc_macro_attribute]
pub fn delete_mapping(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::route_mapping_impl("delete", attr, item)
}

/// PATCH 路由映射
#[proc_macro_attribute]
pub fn patch_mapping(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::route_mapping_impl("patch", attr, item)
}

/// 通用路由映射 - 匹配所有 HTTP 方法
///
/// 当用于方法级别时，表示该路由接受所有 HTTP 请求类型
///
/// # 示例
///
/// ```ignore
/// #[request_mapping("/health")]
/// async fn health_check(&self) -> impl IntoResponse {
///     ResponseEntity::ok("OK")
/// }
/// ```
#[proc_macro_attribute]
pub fn request_mapping(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::route_mapping_impl("any", attr, item)
}

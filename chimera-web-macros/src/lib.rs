//! Chimera Web Macros
//!
//! 提供 Web 相关的过程宏，类似 Spring MVC 的注解

mod exception_handler;
mod route;
mod utils;

use proc_macro::TokenStream;

/// ExceptionHandler 宏
///
/// 将结构体标记为全局异常处理器，自动注册到异常处理器注册表
///
/// # 示例
///
/// ```ignore
/// #[derive(ExceptionHandler, Component)]
/// #[bean("businessExceptionHandler")]
/// struct BusinessExceptionHandler {
///     #[value("app.debug", default = "false")]
///     debug_mode: bool,
/// }
///
/// #[async_trait]
/// impl GlobalExceptionHandler for BusinessExceptionHandler {
///     fn name(&self) -> &str { "BusinessExceptionHandler" }
///     fn priority(&self) -> i32 { 10 }
///     // ...
/// }
/// ```
#[proc_macro_derive(ExceptionHandler)]
pub fn derive_exception_handler(input: TokenStream) -> TokenStream {
    exception_handler::derive_exception_handler(input)
}

/// controller 宏
///
/// 可以用于结构体或 impl 块：
///
/// 1. 用于结构体时：标记为控制器并注册路由，需要提供基础路径参数
/// 2. 用于 impl 块时：扫描路由方法并生成路由注册代码
///
/// # 示例
///
/// ```ignore
/// // 用于结构体 - 提供基础路径
/// #[controller("/user")]
/// #[derive(Component, Clone)]
/// struct UserController {
///     #[autowired]
///     service: Arc<UserService>,
/// }
///
/// // 用于 impl 块 - 扫描路由方法
/// #[controller]
/// impl UserController {
///     #[get_mapping("/:id")]
///     async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
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

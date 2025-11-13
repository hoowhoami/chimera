//! ExceptionHandler 宏实现
//!
//! 自动为实现了 GlobalExceptionHandler trait 的结构体生成 inventory 注册代码

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// ExceptionHandler 派生宏
///
/// 自动生成 inventory::submit! 调用，将异常处理器注册到全局注册表
///
/// # 用法
///
/// ```ignore
/// #[derive(ExceptionHandler, Component)]
/// #[bean("businessExceptionHandler")]
/// struct BusinessExceptionHandler {
///     #[value("app.debug")]
///     debug_mode: bool,
/// }
///
/// #[async_trait]
/// impl GlobalExceptionHandler for BusinessExceptionHandler {
///     fn name(&self) -> &str { "BusinessExceptionHandler" }
///     fn priority(&self) -> i32 { 10 }
///     // ... 实现其他方法
/// }
/// ```
pub fn derive_exception_handler(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        // 提交到全局异常处理器注册表
        ::chimera_web::inventory::submit! {
            ::chimera_web::ExceptionHandlerRegistration::new(
                #name_str,
                || Box::new(#name::default()) as Box<dyn ::chimera_web::exception_handler::GlobalExceptionHandler>
            )
        }
    };

    TokenStream::from(expanded)
}

//! HandlerInterceptor 宏实现
//!
//! 自动为实现了 HandlerInterceptor trait 的结构体生成 inventory 注册代码

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// HandlerInterceptor 派生宏
///
/// 自动生成 inventory::submit! 调用，将处理器拦截器注册到全局注册表
///
/// # 用法
///
/// ```ignore
/// #[derive(HandlerInterceptor, Component)]
/// #[bean("authInterceptor")]
/// struct AuthInterceptor {
///     #[value("security.jwt.secret")]
///     jwt_secret: String,
/// }
///
/// #[async_trait]
/// impl HandlerInterceptor for AuthInterceptor {
///     fn name(&self) -> &str { "AuthInterceptor" }
///     fn priority(&self) -> i32 { 100 }
///     // ... 实现其他方法
/// }
/// ```
pub fn derive_handler_interceptor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        // 提交到全局处理器拦截器注册表
        ::chimera_web::inventory::submit! {
            ::chimera_web::HandlerInterceptorRegistration::new(
                #name_str,
                || Box::new(#name::default()) as Box<dyn ::chimera_web::interceptor::HandlerInterceptor>
            )
        }
    };

    TokenStream::from(expanded)
}

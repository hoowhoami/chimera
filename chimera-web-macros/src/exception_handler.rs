//! ExceptionHandler 宏实现
//!
//! 自动为实现了 GlobalExceptionHandler trait 的结构体生成 inventory 注册代码

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// ExceptionHandler 派生宏
///
/// 自动生成 inventory::submit! 调用，将异常处理器注册到全局注册表
/// 注意：需要同时使用 #[derive(Component)] 和 #[bean("beanName")]
///
/// # 用法
///
/// ```ignore
/// #[derive(ExceptionHandler, Component)]
/// #[bean("businessExceptionHandler")]
/// struct BusinessExceptionHandler {
///     #[value("app.debug", default = false)]
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

    // 从 #[bean("...")] 属性中提取 bean 名称
    let bean_name = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("bean"))
        .and_then(|attr| {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let tokens_str = meta_list.tokens.to_string();
                // 移除引号
                Some(tokens_str.trim_matches('"').to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            // 如果没有 #[bean] 属性，使用类型名的 camelCase 形式
            let mut chars = name_str.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
            }
        });

    let expanded = quote! {
        // 提交到全局异常处理器注册表
        ::chimera_web::inventory::submit! {
            ::chimera_web::ExceptionHandlerRegistration::new(
                #name_str,
                #bean_name,
                |bean: ::std::sync::Arc<dyn ::std::any::Any + Send + Sync>| {
                    // 尝试downcast到具体类型
                    bean.downcast::<#name>()
                        .ok()
                        .map(|concrete| concrete as ::std::sync::Arc<dyn ::chimera_web::exception_handler::GlobalExceptionHandler>)
                }
            )
        }
    };

    TokenStream::from(expanded)
}

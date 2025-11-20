//! Component 属性宏实现
//!
//! 用于标记 Component 的 impl 块，检查方法名是否与 Component trait 冲突

use proc_macro::TokenStream;
use proc_macro_error::abort;
use syn::{ImplItem, ImplItemFn, ItemImpl};

/// Component impl 块属性宏
///
/// 用于标记 Component 类型的 impl 块，自动检查方法名是否与 Component trait 的保留方法冲突
///
/// # 示例
///
/// ```ignore
/// #[derive(Component)]
/// struct UserService { }
///
/// #[component]
/// impl UserService {
///     pub fn create_user(&self) { }  // OK
///     pub fn register(&self) { }     // 编译错误：与 Component::register 冲突
/// }
/// ```
pub fn component_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemImpl);

    // 检查所有方法名
    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            check_reserved_method_name(method);
        }
    }

    // 返回原始的 impl 块（不做任何修改）
    TokenStream::from(quote::quote! { #input })
}

/// Component trait 保留的方法名
const RESERVED_METHODS: &[&str] = &[
    "bean_name",
    "scope",
    "lazy",
    "dependencies",
    "init_callback",
    "destroy_callback",
    "is_event_listener",
    "as_event_listener",
    "create_from_context",
    "register",
];

/// 检查方法名是否与 Component trait 的保留方法冲突
fn check_reserved_method_name(method: &ImplItemFn) {
    let method_name = &method.sig.ident;
    let method_name_str = method_name.to_string();

    if RESERVED_METHODS.contains(&method_name_str.as_str()) {
        abort!(
            method_name.span(),
            "method name '{}' conflicts with Component trait's reserved method",
            method_name_str;
            help = "consider renaming to 'user_{}', '{}_impl', or '{}_handler'",
            method_name_str, method_name_str, method_name_str
        );
    }
}

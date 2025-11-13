use syn::Attribute;
use quote::quote;

/// 从属性中提取bean名称
pub(crate) fn get_bean_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("bean") {
            if let Ok(name_lit) = attr.parse_args::<syn::LitStr>() {
                return Some(name_lit.value());
            }
        }
    }
    None
}

/// 从属性中提取作用域
pub(crate) fn get_scope(attrs: &[Attribute]) -> proc_macro2::TokenStream {
    for attr in attrs {
        if attr.path().is_ident("scope") {
            if let Ok(scope_lit) = attr.parse_args::<syn::LitStr>() {
                let scope_value = scope_lit.value();
                return match scope_value.as_str() {
                    "singleton" => quote! { chimera_core::Scope::Singleton },
                    "prototype" => quote! { chimera_core::Scope::Prototype },
                    _ => quote! { chimera_core::Scope::Singleton },
                };
            }
        }
    }
    quote! { chimera_core::Scope::Singleton }
}

/// 从属性中提取是否延迟初始化
pub(crate) fn get_lazy(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("lazy") {
            return true;
        }
    }
    false
}

/// 将 PascalCase 转换为 camelCase
pub(crate) fn to_camel_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

/// 从属性中提取 init 方法名
/// 支持格式: #[init("custom_method")] 或 #[init]（默认使用 init）
pub(crate) fn get_init_method(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("init") {
            // 尝试解析参数
            if let Ok(method_lit) = attr.parse_args::<syn::LitStr>() {
                return Some(method_lit.value());
            }
            // 没有参数，使用默认方法名
            return Some("init".to_string());
        }
    }
    None
}

/// 从属性中提取 destroy 方法名
/// 支持格式: #[destroy("custom_method")] 或 #[destroy]（默认使用 destroy）
pub(crate) fn get_destroy_method(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("destroy") {
            // 尝试解析参数
            if let Ok(method_lit) = attr.parse_args::<syn::LitStr>() {
                return Some(method_lit.value());
            }
            // 没有参数，使用默认方法名
            return Some("destroy".to_string());
        }
    }
    None
}

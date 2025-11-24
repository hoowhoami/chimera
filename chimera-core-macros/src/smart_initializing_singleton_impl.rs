use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_smart_initializing_singleton_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // 生成 bean 名称（camelCase）
    let bean_name = {
        let name_str = struct_name.to_string();
        let mut chars = name_str.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        }
    };

    let expanded = quote! {
        // 提交 SmartInitializingSingleton 标记到 inventory
        ::chimera_core::inventory::submit! {
            ::chimera_core::SmartInitializingSingletonMarker {
                bean_name: #bean_name,
                type_name: ::std::stringify!(#struct_name),
                getter: |ctx: &::std::sync::Arc<::chimera_core::ApplicationContext>| {
                    let bean = ctx.get_bean_by_type::<#struct_name>()?;
                    Ok(bean as ::std::sync::Arc<dyn ::chimera_core::SmartInitializingSingleton>)
                },
            }
        }
    };

    TokenStream::from(expanded)
}


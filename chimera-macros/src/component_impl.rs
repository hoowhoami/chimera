use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

use crate::attribute_helpers::{
    get_bean_name, get_destroy_method, get_init_method, get_lazy, get_scope, to_camel_case,
};
use crate::value_injection::get_value_info;

pub(crate) fn derive_component_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let bean_name = get_bean_name(&input.attrs).unwrap_or_else(|| {
        // 默认使用类型名的 camelCase 形式
        // 与 chimera_core::utils::naming::to_camel_case 的逻辑保持一致
        // 例如: UserService -> userService
        let name_str = name.to_string();
        let mut chars = name_str.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        }
    });

    let scope = get_scope(&input.attrs);
    let lazy = get_lazy(&input.attrs);
    let init_method = get_init_method(&input.attrs);
    let destroy_method = get_destroy_method(&input.attrs);

    // 检查是否有 #[event_listener] 属性
    let is_event_listener = input
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("event_listener"));

    // 获取所有字段
    let all_fields = if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields) = &data_struct.fields {
            fields.named.iter().collect::<Vec<_>>()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // 提取需要自动注入的字段
    let autowired_fields: Vec<_> = all_fields
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("autowired")))
        .copied()
        .collect();

    // 提取需要配置注入的字段
    let value_fields: Vec<_> = all_fields
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("value")))
        .copied()
        .collect();

    // 生成autowired字段注入代码
    let autowired_injections = autowired_fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;

        // 提取Arc<T>中的T类型
        let inner_type = extract_arc_type(field_type);

        // 检查是否指定了特定的bean名称
        let bean_name = get_autowired_bean_name(&field.attrs);

        if let Some(bean_name) = bean_name {
            // 使用集中的核心组件注入逻辑
            generate_core_component_injection(field_type, &field_name, &bean_name)
        } else {
            // 检查是否为核心组件类型，即使没有指定bean名称也要特殊处理
            if is_core_component_type_id(&inner_type) {
                // 是核心组件，使用 CoreComponent trait 的特殊注入方式
                // CoreComponent::get_from_context 返回 Arc<Self>，正好匹配字段类型
                quote! {
                    let #field_name = <#inner_type as chimera_core::CoreComponent>::get_from_context(&context);
                }
            } else {
                // 使用类型注入
                quote! {
                    let #field_name = context.get_bean_by_type::<#inner_type>().await?;
                }
            }
        }
    });

    // 生成value字段注入代码
    let value_injections = value_fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;

        if let Some(value_info) = get_value_info(&field.attrs) {
            let config_key = value_info.key;
            let default_value = value_info.default_value;

            // 根据类型生成不同的转换代码
            let type_str = quote! { #field_type }.to_string();

            if default_value.is_some() {
                let default = default_value.unwrap();
                // 有默认值的情况
                if type_str.contains("String") {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_string(#config_key)
                            .unwrap_or_else(|| #default.to_string());
                    }
                } else if type_str.contains("i64")
                    || type_str.contains("i32")
                    || type_str.contains("u64")
                    || type_str.contains("u32")
                {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_i64(#config_key)
                            .unwrap_or(#default) as #field_type;
                    }
                } else if type_str.contains("f64") || type_str.contains("f32") {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_f64(#config_key)
                            .unwrap_or(#default) as #field_type;
                    }
                } else if type_str.contains("bool") {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_bool(#config_key)
                            .unwrap_or(#default);
                    }
                } else {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_string(#config_key)
                            .unwrap_or_else(|| #default.to_string())
                            .parse()
                            .map_err(|e| chimera_core::ContainerError::Custom(
                                format!("Failed to parse config '{}': {}", #config_key, e)
                            ))?;
                    }
                }
            } else {
                // 没有默认值的情况，配置必须存在
                if type_str.contains("String") {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_string(#config_key)
                            .ok_or_else(|| chimera_core::ContainerError::Custom(
                                format!("Required config '{}' not found", #config_key)
                            ))?;
                    }
                } else if type_str.contains("i64")
                    || type_str.contains("i32")
                    || type_str.contains("u64")
                    || type_str.contains("u32")
                {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_i64(#config_key)
                            .ok_or_else(|| chimera_core::ContainerError::Custom(
                                format!("Required config '{}' not found", #config_key)
                            ))? as #field_type;
                    }
                } else if type_str.contains("f64") || type_str.contains("f32") {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_f64(#config_key)
                            .ok_or_else(|| chimera_core::ContainerError::Custom(
                                format!("Required config '{}' not found", #config_key)
                            ))? as #field_type;
                    }
                } else if type_str.contains("bool") {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_bool(#config_key)
                            .ok_or_else(|| chimera_core::ContainerError::Custom(
                                format!("Required config '{}' not found", #config_key)
                            ))?;
                    }
                } else {
                    quote! {
                        let #field_name = context.get_environment()
                            .get_string(#config_key)
                            .ok_or_else(|| chimera_core::ContainerError::Custom(
                                format!("Required config '{}' not found", #config_key)
                            ))?
                            .parse()
                            .map_err(|e| chimera_core::ContainerError::Custom(
                                format!("Failed to parse config '{}': {}", #config_key, e)
                            ))?;
                    }
                }
            }
        } else {
            quote! {}
        }
    });

    // 收集所有字段名
    let field_names: Vec<_> = all_fields.iter().map(|f| &f.ident).collect();

    // 生成依赖列表（从 Arc<T> 提取 T 的类型名，转换为 bean 名称）
    // 但排除核心组件，因为它们有特殊的注入方式
    let dependency_names: Vec<String> = autowired_fields
        .iter()
        .filter_map(|field| {
            // 检查是否指定了特定的bean名称
            let bean_name = get_autowired_bean_name(&field.attrs);

            if let Some(bean_name) = bean_name {
                // 检查是否为核心组件类型
                let inner_type = extract_arc_type(&field.ty);

                if is_core_component_type_id(&inner_type) {
                    // 核心组件不包含在依赖列表中
                    None
                } else {
                    Some(bean_name)
                }
            } else {
                // 使用类型注入的情况
                let inner_type = extract_arc_type(&field.ty);

                // 检查是否为核心组件
                if is_core_component_type_id(&inner_type) {
                    None
                } else {
                    let type_name = quote! { #inner_type }.to_string();
                    // 将类型名转换为 camelCase bean 名称
                    Some(to_camel_case(&type_name))
                }
            }
        })
        .collect();

    // 生成 init_callback 和 destroy_callback 实现
    let init_callback_impl = if let Some(method_name) = &init_method {
        let method_ident = syn::Ident::new(method_name, proc_macro2::Span::call_site());
        quote! {
            fn init_callback() -> Option<fn(&mut Self) -> chimera_core::ContainerResult<()>> {
                Some(Self::#method_ident)
            }
        }
    } else {
        quote! {}
    };

    let destroy_callback_impl = if let Some(method_name) = &destroy_method {
        let method_ident = syn::Ident::new(method_name, proc_macro2::Span::call_site());
        quote! {
            fn destroy_callback() -> Option<fn(&mut Self) -> chimera_core::ContainerResult<()>> {
                Some(Self::#method_ident)
            }
        }
    } else {
        quote! {}
    };

    // 生成Component trait实现和自动注册代码（异步）
    let event_listener_registration = if is_event_listener {
        quote! {
            // 自动向inventory注册EventListener
            inventory::submit! {
                chimera_core::EventListenerRegistry {
                    registrar: |ctx: &std::sync::Arc<chimera_core::ApplicationContext>| {
                        let ctx = std::sync::Arc::clone(ctx);
                        Box::pin(async move {
                            let listener = ctx.get_bean_by_type::<#name>().await?;
                            Ok(listener as std::sync::Arc<dyn chimera_core::EventListener>)
                        })
                    },
                    name: #bean_name,
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #[::chimera_core::async_trait::async_trait]
        impl chimera_core::Component for #name {
            fn bean_name() -> &'static str {
                #bean_name
            }

            fn scope() -> chimera_core::Scope {
                #scope
            }

            fn lazy() -> bool {
                #lazy
            }

            fn dependencies() -> Vec<String> {
                vec![#(#dependency_names.to_string()),*]
            }

            #init_callback_impl

            #destroy_callback_impl

            async fn create_from_context(context: &std::sync::Arc<chimera_core::ApplicationContext>) -> chimera_core::ContainerResult<Self> {
                use std::sync::Arc;

                #(#autowired_injections)*
                #(#value_injections)*

                Ok(Self {
                    #(#field_names),*
                })
            }
        }

        // 自动向inventory注册Component
        inventory::submit! {
            chimera_core::component::ComponentRegistry {
                registrar: |ctx: &std::sync::Arc<chimera_core::ApplicationContext>| {
                    let ctx = std::sync::Arc::clone(ctx);
                    Box::pin(async move {
                        #name::register(&ctx).await
                    })
                },
                name: #bean_name,
            }
        }

        // 条件性注册EventListener
        #event_listener_registration
    };

    TokenStream::from(expanded)
}

/// 辅助函数：从Arc<T>类型中提取T
fn extract_arc_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Arc" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return inner_ty;
                    }
                }
            }
        }
    }
    ty
}

/// 从 #[autowired] 或 #[autowired("beanName")] 中提取bean名称
fn get_autowired_bean_name(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("autowired") {
            if let syn::Meta::List(meta_list) = &attr.meta {
                // #[autowired("beanName")] 格式
                let tokens = &meta_list.tokens;
                let tokens_str = tokens.to_string();
                if !tokens_str.is_empty() {
                    // 去掉引号
                    return Some(tokens_str.trim_matches('"').to_string());
                }
            }
            // 只有 #[autowired] 没有参数的情况返回 None
        }
    }
    None
}

/// 生成核心组件检查和注入代码
/// 这个版本将所有核心组件逻辑集中到一个地方，比原来的分散硬编码更优雅
/// 当需要添加新的核心组件时，只需要在这一个函数中添加即可
fn generate_core_component_injection(
    type_for_injection: &syn::Type,
    field_name: &Option<syn::Ident>,
    bean_name: &str,
) -> proc_macro2::TokenStream {
    // 提取内部类型以检查是否为核心组件
    let inner_type = extract_arc_type(type_for_injection);

    // 检查是否为核心组件类型（编译时检查）
    if is_core_component_type_id(&inner_type) {
        // 是核心组件，使用 CoreComponent trait 的特殊注入方式
        // CoreComponent::get_from_context 返回 Arc<Self>，匹配 Arc<T> 字段类型
        quote! {
            let #field_name = <#inner_type as chimera_core::CoreComponent>::get_from_context(&context);
        }
    } else {
        // 不是核心组件，使用普通的 bean 查找
        quote! {
            let #field_name = {
                let bean_any = context.get_bean(#bean_name).await?;
                bean_any.downcast::<#type_for_injection>()
                    .map_err(|_| chimera_core::ContainerError::TypeMismatch {
                        expected: std::any::type_name::<#type_for_injection>().to_string(),
                        found: "unknown".to_string(),
                    })?
            };
        }
    }
}

/// 检查类型是否为核心组件
/// 直接使用 chimera_core 中的统一检查函数
fn is_core_component_type_id(inner_type: &syn::Type) -> bool {
    let type_tokens = quote! { #inner_type }.to_string();

    // 直接使用 chimera_core 中定义的检查函数，避免重复定义
    chimera_core::constants::is_core_component_type_name(&type_tokens)
}

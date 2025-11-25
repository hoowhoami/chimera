//! Configuration impl block attribute macro
//! 
//! 用于 Configuration 类的 impl 块，自动扫描和注册所有 #[bean] 方法

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, ItemImpl, ImplItem, ImplItemFn, ReturnType, Type};

/// Configuration impl 块属性宏
///
/// 处理 impl 块中所有 #[bean] 标记的方法并自动注册
pub(crate) fn configuration_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    
    // 获取类型名
    let self_ty = &input.self_ty;
    
    // 收集所有 #[bean] 方法
    let bean_methods: Vec<_> = input.items.iter().filter_map(|item| {
        if let ImplItem::Fn(method) = item {
            // 检查是否有 #[bean] 属性
            let bean_attr = method.attrs.iter().find(|attr| attr.path().is_ident("bean"));
            if bean_attr.is_some() {
                Some(method)
            } else {
                None
            }
        } else {
            None
        }
    }).collect();
    
    // 生成注册代码
    let registrations = bean_methods.iter().map(|method| {
        let method_name = &method.sig.ident;
        let bean_name = extract_bean_name(&method.attrs, &method_name.to_string());
        let scope = extract_scope(&method.attrs);
        let is_lazy = extract_lazy(&method.attrs);
        let init_method = extract_init(&method.attrs);
        let destroy_method = extract_destroy(&method.attrs);

        // 检查返回类型
        let (return_type, is_result) = match &method.sig.output {
            ReturnType::Default => {
                return syn::Error::new_spanned(
                    method,
                    "#[bean] method must return a value"
                ).to_compile_error();
            }
            ReturnType::Type(_, ty) => {
                if let Some(inner) = extract_result_type(ty) {
                    (inner, true)
                } else {
                    (ty.as_ref(), false)
                }
            }
        };
        
        // Generate different code based on whether method returns Result or not
        let bean_creation = if is_result {
            // Method returns Result<T, E>, use ? to propagate errors
            quote! {
                let result = config_ref.#method_name();
                let bean_instance: #return_type = result?;
            }
        } else {
            // Method returns T directly, no need for ? operator
            quote! {
                let bean_instance: #return_type = config_ref.#method_name();
            }
        };

        // 生成 init 回调代码
        let init_callback_code = if let Some(ref init_method_name) = init_method {
            let init_ident = syn::Ident::new(init_method_name, proc_macro2::Span::call_site());
            quote! {
                let definition = definition.with_init(|bean: &mut dyn std::any::Any| -> chimera_core::ContainerResult<()> {
                    if let Some(instance) = bean.downcast_mut::<#return_type>() {
                        instance.#init_ident()?;
                    }
                    Ok(())
                });
            }
        } else {
            quote! {}
        };

        // 生成 destroy 回调代码
        let destroy_callback_code = if let Some(ref destroy_method_name) = destroy_method {
            let destroy_ident = syn::Ident::new(destroy_method_name, proc_macro2::Span::call_site());
            quote! {
                let definition = definition.with_destroy(|bean: &mut dyn std::any::Any| -> chimera_core::ContainerResult<()> {
                    if let Some(instance) = bean.downcast_mut::<#return_type>() {
                        instance.#destroy_ident()?;
                    }
                    Ok(())
                });
            }
        } else {
            quote! {}
        };

        let scope_code = match scope.as_str() {
            "prototype" => quote! { chimera_core::Scope::Prototype },
            _ => quote! { chimera_core::Scope::Singleton },
        };

        let register_fn_name = syn::Ident::new(
            &format!("__register_bean_{}_{}", bean_name.replace("-", "_"), method_name),
            proc_macro2::Span::call_site(),
        );

        // 获取类型名的字符串形式
        let self_ty_str = quote! { #self_ty }.to_string();

        // 为每个 bean 方法生成注册代码
        quote! {
            #[allow(non_snake_case)]
            fn #register_fn_name(
                context: &std::sync::Arc<chimera_core::ApplicationContext>,
                config_instance: std::sync::Arc<dyn std::any::Any + Send + Sync>,
            ) -> chimera_core::ContainerResult<()> {
                use chimera_core::Container;

                let ctx = std::sync::Arc::clone(context);
                let config = config_instance.clone();

                // Factory 闭包直接返回具体类型，让 FunctionFactory 推断正确的 TypeId
                let factory = move || -> chimera_core::ContainerResult<#return_type> {
                    let config_ref = config.clone()
                        .downcast::<#self_ty>()
                        .map_err(|_e: std::sync::Arc<dyn std::any::Any + Send + Sync>| {
                            chimera_core::ContainerError::BeanCreationFailed(
                                format!("Failed to downcast config instance for bean '{}'", #bean_name)
                            )
                        })?;

                    #bean_creation
                    Ok(bean_instance)
                };

                let definition = chimera_core::BeanDefinition::new(
                    #bean_name,
                    chimera_core::bean::FunctionFactory::new(factory),
                )
                .with_scope(#scope_code);

                let definition = if #is_lazy {
                    definition.with_lazy(true)
                } else {
                    definition
                };

                // 添加 init 回调（如果指定了 init 方法）
                #init_callback_code

                // 添加 destroy 回调（如果指定了 destroy 方法）
                #destroy_callback_code

                ctx.as_ref().register(definition)?;
                Ok(())
            }

            inventory::submit! {
                chimera_core::bean::BeanMethodRegistry {
                    registrar: #register_fn_name,
                    bean_name: #bean_name,
                    config_type_name: #self_ty_str,
                }
            }
        }
    });
    
    // 生成最终代码
    let expanded = quote! {
        #input
        
        // 注册所有 bean 方法
        #(#registrations)*
    };
    
    TokenStream::from(expanded)
}

/// 提取 bean 名称
fn extract_bean_name(attrs: &[Attribute], default_name: &str) -> String {
    for attr in attrs {
        if attr.path().is_ident("bean") {
            if let Ok(lit) = attr.parse_args::<syn::LitStr>() {
                return lit.value();
            }
        }
    }
    default_name.to_string()
}

/// 提取 scope
fn extract_scope(attrs: &[Attribute]) -> String {
    for attr in attrs {
        if attr.path().is_ident("scope") {
            if let Ok(lit) = attr.parse_args::<syn::LitStr>() {
                return lit.value();
            }
        }
    }
    "singleton".to_string()
}

/// 提取 lazy 标记
fn extract_lazy(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("lazy"))
}

/// 提取 init 方法名
fn extract_init(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("init") {
            // #[init("method_name")] 或 #[init] (默认为 "init")
            if let Ok(lit) = attr.parse_args::<syn::LitStr>() {
                return Some(lit.value());
            } else {
                // 如果没有参数，默认方法名为 "init"
                return Some("init".to_string());
            }
        }
    }
    None
}

/// 提取 destroy 方法名
fn extract_destroy(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("destroy") {
            // #[destroy("method_name")] 或 #[destroy] (默认为 "destroy")
            if let Ok(lit) = attr.parse_args::<syn::LitStr>() {
                return Some(lit.value());
            } else {
                // 如果没有参数，默认方法名为 "destroy"
                return Some("destroy".to_string());
            }
        }
    }
    None
}

/// 从 Result<T, E> 中提取 T
fn extract_result_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Result" || segment.ident == "ContainerResult" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

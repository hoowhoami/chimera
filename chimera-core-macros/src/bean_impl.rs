use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, FnArg, ItemFn, Lit, ReturnType, Type};

/// Bean 方法宏实现
///
/// 支持以下格式：
/// - #[bean] - 使用方法名作为 bean 名称
/// - #[bean("customName")] - 指定自定义 bean 名称
pub(crate) fn bean_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    // 解析 bean 名称（只支持字符串字面量）
    let bean_name = if attr.is_empty() {
        // 如果没有参数，使用方法名
        input.sig.ident.to_string()
    } else {
        // 尝试解析字符串字面量
        match syn::parse::<Lit>(attr.clone()) {
            Ok(Lit::Str(s)) => s.value(),
            Ok(_) => {
                return syn::Error::new_spanned(
                    &input.sig,
                    "#[bean] only accepts a string literal, e.g., #[bean(\"customName\")]",
                )
                .to_compile_error()
                .into();
            }
            Err(e) => return e.to_compile_error().into(),
        }
    };

    // 从方法的属性中提取 scope
    let scope = extract_scope_from_attributes(&input.attrs);

    // 获取方法的基本信息
    let method_name = &input.sig.ident;
    let method_vis = &input.vis;
    let method_body = &input.block;
    let method_inputs = &input.sig.inputs;
    let method_output = &input.sig.output;

    // 提取返回类型，并判断是否为 Result 类型
    let (return_type, is_result_type) = match method_output {
        ReturnType::Default => {
            return syn::Error::new_spanned(
                &input.sig,
                "#[bean] method must return a value",
            )
            .to_compile_error()
            .into();
        }
        ReturnType::Type(_, ty) => {
            // 尝试提取 Result<T> 中的 T
            if let Some(inner) = extract_result_type(ty) {
                (inner, true)
            } else {
                // 不是 Result 类型，直接使用原类型
                (ty.as_ref(), false)
            }
        }
    };

    // 生成包装代码：如果不是 Result 类型，自动包装成 Ok(T)
    let result_wrapper = if is_result_type {
        // 已经是 Result 类型，直接使用
        quote! { result }
    } else {
        // 不是 Result 类型，包装成 Ok(T)
        quote! { Ok(result) }
    };

    // 生成作用域代码
    let scope_code = match scope.as_str() {
        "prototype" => quote! { chimera_core::Scope::Prototype },
        "singleton" | _ => quote! { chimera_core::Scope::Singleton },
    };

    // 检查方法是否有 self 参数
    let has_self = method_inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_)));

    if !has_self {
        return syn::Error::new_spanned(
            &input.sig,
            "#[bean] method must have &self parameter",
        )
        .to_compile_error()
        .into();
    }

    // 原始方法保持不变
    let original_method = quote! {
        #method_vis fn #method_name(#method_inputs) #method_output {
            #method_body
        }
    };

    // 生成一个唯一的注册函数名
    let register_fn_name = syn::Ident::new(
        &format!(
            "__register_bean_{}_{}",
            bean_name.replace("-", "_").replace(" ", "_"),
            method_name
        ),
        proc_macro2::Span::call_site(),
    );

    // 获取当前结构体类型（Self）
    // 注意：这个宏必须在 impl 块内的方法上使用
    // 由于 inventory::submit! 需要在全局作用域，我们需要生成一个唯一的标识
    let struct_type_for_inventory = format!("__bean_config_type_{}_{}", bean_name.replace("-", "_").replace(" ", "_"), method_name);
    let inventory_const_name = syn::Ident::new(
        &format!("__BEAN_REGISTRY_{}_{}", bean_name.replace("-", "_").replace(" ", "_").to_uppercase(), method_name.to_string().to_uppercase()),
        proc_macro2::Span::call_site(),
    );

    // 生成 bean 注册代码
    let expanded = quote! {
        #original_method

        // 生成自动注册函数（在全局作用域）
        #[allow(non_snake_case, non_upper_case_globals)]
        const #inventory_const_name: () = {
            // 定义注册函数
            fn #register_fn_name(
                context: &std::sync::Arc<chimera_core::ApplicationContext>,
                config_instance: std::sync::Arc<dyn std::any::Any + Send + Sync>,
            ) -> chimera_core::ContainerResult<()> {
                use chimera_core::Container;

                // 创建工厂函数，捕获配置实例
                let ctx = std::sync::Arc::clone(context);
                let config = config_instance.clone();

                let factory = move || -> chimera_core::ContainerResult<Box<dyn std::any::Any + Send + Sync>> {
                    // downcast 到具体的配置类型
                    // 注意：这里我们使用动态类型，因为在编译时无法确定具体类型
                    // 配置类会在运行时传入
                    let config_any = config.clone();

                    // 这是一个占位符，实际的类型转换会在运行时进行
                    // 我们需要另一种方式来处理这个问题
                    unimplemented!("Bean method registration needs refactoring")
                };

                // 注册 BeanDefinition
                let definition = chimera_core::BeanDefinition::new(
                    #bean_name,
                    chimera_core::bean::FunctionFactory::new(factory),
                )
                .with_scope(#scope_code);

                ctx.as_ref().register(definition)?;
                Ok(())
            }

            // 使用 inventory 收集 bean 注册信息
            inventory::submit! {
                chimera_core::bean::BeanMethodRegistry {
                    registrar: #register_fn_name,
                    bean_name: #bean_name,
                    config_type_name: "", // 临时占位符
                }
            }
        };
    };

    TokenStream::from(expanded)
}

/// 从方法的属性中提取 scope
fn extract_scope_from_attributes(attrs: &[Attribute]) -> String {
    for attr in attrs {
        if attr.path().is_ident("scope") {
            if let Ok(scope_lit) = attr.parse_args::<syn::LitStr>() {
                return scope_lit.value();
            }
        }
    }
    "singleton".to_string()
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


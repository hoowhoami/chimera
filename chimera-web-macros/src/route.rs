//! 路由相关宏实现

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, FnArg, ImplItem, ImplItemFn, ItemImpl, ItemStruct, Type};

/// controller 宏实现
///
/// 可以用于结构体或 impl 块：
/// - 用于结构体：提取路径并注册到全局
/// - 用于 impl 块：扫描路由方法并生成路由注册代码
pub fn controller_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_clone = item.clone();

    // 尝试解析为结构体
    if let Ok(item_struct) = syn::parse::<ItemStruct>(item_clone.clone()) {
        return handle_controller_struct(attr, item_struct);
    }

    // 尝试解析为 impl 块
    if let Ok(item_impl) = syn::parse::<ItemImpl>(item_clone) {
        return handle_controller_impl(attr, item_impl);
    }

    // 都不是，返回错误
    let error = quote! {
        compile_error!("controller macro can only be used on struct or impl block");
    };
    TokenStream::from(error)
}

/// 处理结构体上的 controller 宏
///
/// 生成：
/// - __base_path() 方法
/// - 全局注册代码
fn handle_controller_struct(attr: TokenStream, item: ItemStruct) -> TokenStream {
    let name = &item.ident;

    // 解析路径参数
    let base_path = if attr.is_empty() {
        "/".to_string()
    } else {
        // 解析路径字符串
        let attr_str = attr.to_string();
        attr_str.trim_matches('"').to_string()
    };

    let expanded = quote! {
        #item

        impl #name {
            pub fn __base_path() -> &'static str {
                #base_path
            }
        }

        // 提交到全局控制器注册表
        ::chimera_core::inventory::submit! {
            ::chimera_web::controller::ControllerRegistration {
                type_name: stringify!(#name),
                base_path: #base_path,
                register: |router| {
                    #name::__register_routes(router)
                },
                get_route_list: || #name::__get_routes(),
            }
        }
    };

    TokenStream::from(expanded)
}

/// 处理 impl 块上的 controller 宏
///
/// 扫描路由方法并生成路由注册代码
fn handle_controller_impl(_attr: TokenStream, input: ItemImpl) -> TokenStream {
    let self_ty = &input.self_ty;

    // 收集所有路由方法
    let mut route_registrations = Vec::new();
    let mut route_info_list = Vec::new(); // 用于收集路由信息

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            if let Some((http_method, path)) = extract_route_info(method) {
                let method_name = &method.sig.ident;

                // 收集路由信息用于冲突检测
                route_info_list.push((http_method.to_string(), path.clone()));

                // 提取所有方法参数（跳过 &self）
                let params = extract_method_params(method);

                // 生成路由处理函数
                if params.is_empty() {
                    // 无参数的情况
                    let handler = quote! {
                        let full_path = format!("{}{}", #self_ty::__base_path(), #path);
                        router = router.route(
                            &full_path,
                            ::chimera_web::prelude::axum::routing::#http_method(|
                                ::chimera_web::prelude::axum::Extension(context):
                                    ::chimera_web::prelude::axum::Extension<::std::sync::Arc<::chimera_core::ApplicationContext>>,
                            | async move {
                                match context.get_bean_by_type::<#self_ty>() {
                                    Ok(controller) => {
                                        use ::chimera_web::prelude::IntoResponse;
                                        controller.#method_name().await.into_response()
                                    }
                                    Err(e) => {
                                        use ::chimera_web::prelude::IntoResponse;
                                        ::tracing::error!("Failed to get controller bean {}: {:?}", stringify!(#self_ty), e);
                                        ::chimera_web::prelude::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
                                    }
                                }
                            })
                        );
                    };
                    route_registrations.push(handler);
                } else {
                    // 有参数的情况 - 生成带提取器的 handler
                    let param_patterns: Vec<_> = params.iter().map(|p| &p.pattern).collect();
                    let param_types: Vec<_> = params.iter().map(|p| &p.ty).collect();
                    let param_names: Vec<_> = params.iter().map(|p| &p.pattern).collect();

                    let handler = quote! {
                        let full_path = format!("{}{}", #self_ty::__base_path(), #path);
                        router = router.route(
                            &full_path,
                            ::chimera_web::prelude::axum::routing::#http_method({
                                use ::chimera_web::prelude::axum::Extension;
                                use ::std::sync::Arc;
                                use ::chimera_core::ApplicationContext;

                                move |
                                    Extension(context): Extension<Arc<ApplicationContext>>,
                                    #(#param_patterns: #param_types),*
                                | async move {
                                    match context.get_bean_by_type::<#self_ty>() {
                                        Ok(controller) => {
                                            use ::chimera_web::prelude::IntoResponse;
                                            controller.#method_name(#(#param_names),*).await.into_response()
                                        }
                                        Err(e) => {
                                            use ::chimera_web::prelude::IntoResponse;
                                            ::tracing::error!("Failed to get controller bean {}: {:?}", stringify!(#self_ty), e);
                                            ::chimera_web::prelude::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
                                        }
                                    }
                                }
                            })
                        );
                    };
                    route_registrations.push(handler);
                }
            }
        }
    }

    // 生成路由信息数组
    let route_info_tokens: Vec<_> = route_info_list.iter().map(|(method, path)| {
        quote! { (#method, #path) }
    }).collect();

    // 生成代码
    let expanded = quote! {
        #input

        impl #self_ty {
            /// 注册控制器的所有路由
            pub fn __register_routes(
                mut router: ::chimera_web::prelude::axum::Router,
            ) -> ::chimera_web::prelude::axum::Router {
                #(#route_registrations)*
                router
            }

            /// 获取所有路由信息（用于冲突检测）
            pub fn __get_routes() -> &'static [(&'static str, &'static str)] {
                &[#(#route_info_tokens),*]
            }
        }
    };

    TokenStream::from(expanded)
}

/// 方法参数信息
struct MethodParam {
    pattern: syn::Pat,
    ty: Type,
}

/// 提取方法的所有参数（跳过 &self）
fn extract_method_params(method: &ImplItemFn) -> Vec<MethodParam> {
    let mut params = Vec::new();

    for arg in &method.sig.inputs {
        match arg {
            FnArg::Receiver(_) => {
                // 跳过 &self
                continue;
            }
            FnArg::Typed(pat_type) => {
                params.push(MethodParam {
                    pattern: (*pat_type.pat).clone(),
                    ty: (*pat_type.ty).clone(),
                });
            }
        }
    }

    params
}

/// 从方法中提取路由信息
fn extract_route_info(method: &ImplItemFn) -> Option<(syn::Ident, String)> {
    for attr in &method.attrs {
        if let Some(ident) = attr.path().get_ident() {
            let ident_str = ident.to_string();

            let http_method = match ident_str.as_str() {
                "get_mapping" => "get",
                "post_mapping" => "post",
                "put_mapping" => "put",
                "delete_mapping" => "delete",
                "patch_mapping" => "patch",
                "request_mapping" => "any",
                _ => continue,
            };

            // 提取路径
            let path = extract_path_from_attr(attr).unwrap_or_else(|| "/".to_string());
            let method_ident = syn::Ident::new(http_method, ident.span());

            return Some((method_ident, path));
        }
    }
    None
}

/// 从属性中提取路径
fn extract_path_from_attr(attr: &Attribute) -> Option<String> {
    if let syn::Meta::List(meta_list) = &attr.meta {
        // 使用 syn::parse2 来正确解析字符串字面量
        meta_list.tokens.clone().into_iter().next().and_then(|token| {
            // 尝试将 token 解析为 LitStr
            syn::parse2::<syn::LitStr>(token.into())
                .ok()
                .map(|lit| lit.value())
        })
    } else {
        None
    }
}

/// 路由映射宏实现
///
/// 这些宏只是标记，实际的路由注册由 controller_impl 完成
pub fn route_mapping_impl(_method: &str, _attr: TokenStream, item: TokenStream) -> TokenStream {
    // 直接返回原始的方法定义，属性信息会被保留
    item
}

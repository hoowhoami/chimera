//! 路由相关宏实现

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl, ImplItem, ImplItemFn, Attribute, FnArg, Pat};

/// controller_impl 宏实现
///
/// 处理控制器实现块，扫描路由方法并生成路由注册代码
pub fn controller_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let self_ty = &input.self_ty;

    // 收集所有路由方法
    let mut route_registrations = Vec::new();

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            if let Some((http_method, path)) = extract_route_info(method) {
                let method_name = &method.sig.ident;

                // 从路径中提取参数（如 /users/:id<\d+> -> [PathParam { name: "id", pattern: Some(r"\d+") }]）
                let path_params = extract_path_params_from_path(&path);

                // 提取函数参数（跳过 &self）
                let fn_params = extract_fn_params(method);

                // 将路径转换为 Axum 格式（移除正则表达式部分）
                let axum_path = convert_path_to_axum(&path);

                // 生成路由处理函数
                let handler = if path_params.is_empty() {
                    // 没有路径参数的情况
                    quote! {
                        // 拼接完整路径: base_path + method_path
                        let full_path = format!("{}{}", #self_ty::__base_path(), #axum_path);
                        router = router.route(
                            &full_path,
                            ::chimera_web::prelude::axum::routing::#http_method(|
                                ::chimera_web::prelude::axum::Extension(context):
                                    ::chimera_web::prelude::axum::Extension<::std::sync::Arc<::chimera_core::ApplicationContext>>,
                            | async move {
                                // 从ApplicationContext获取controller bean
                                match context.get_bean_by_type::<#self_ty>().await {
                                    Ok(controller) => {
                                        use ::chimera_web::prelude::IntoResponse;
                                        controller.#method_name().await.into_response()
                                    }
                                    Err(_) => {
                                        use ::chimera_web::prelude::IntoResponse;
                                        (
                                            ::chimera_web::prelude::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                            "Controller bean not found"
                                        ).into_response()
                                    }
                                }
                            })
                        );
                    }
                } else if path_params.len() == 1 && !fn_params.is_empty() {
                    // 单个路径参数的情况
                    let param_name = &fn_params[0];
                    let path_param = &path_params[0];

                    if let Some(pattern) = &path_param.pattern {
                        // 带正则验证的参数
                        quote! {
                            // 拼接完整路径: base_path + method_path
                            let full_path = format!("{}{}", #self_ty::__base_path(), #axum_path);
                            router = router.route(
                                &full_path,
                                ::chimera_web::prelude::axum::routing::#http_method(|
                                    ::chimera_web::prelude::axum::Extension(context):
                                        ::chimera_web::prelude::axum::Extension<::std::sync::Arc<::chimera_core::ApplicationContext>>,
                                    ::chimera_web::prelude::axum::extract::Path(#param_name):
                                        ::chimera_web::prelude::axum::extract::Path<String>,
                                | async move {
                                    // 验证参数
                                    let re = ::regex::Regex::new(#pattern).unwrap();
                                    if !re.is_match(&#param_name) {
                                        // 验证失败，返回 404
                                        use ::chimera_web::prelude::IntoResponse;
                                        return (
                                            ::chimera_web::prelude::axum::http::StatusCode::NOT_FOUND,
                                            "Not Found"
                                        ).into_response();
                                    }

                                    // 从ApplicationContext获取controller bean
                                    match context.get_bean_by_type::<#self_ty>().await {
                                        Ok(controller) => {
                                            use ::chimera_web::prelude::IntoResponse;
                                            controller.#method_name(#param_name).await.into_response()
                                        }
                                        Err(_) => {
                                            use ::chimera_web::prelude::IntoResponse;
                                            (
                                                ::chimera_web::prelude::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                                "Controller bean not found"
                                            ).into_response()
                                        }
                                    }
                                })
                            );
                        }
                    } else {
                        // 无正则验证的参数
                        quote! {
                            // 拼接完整路径: base_path + method_path
                            let full_path = format!("{}{}", #self_ty::__base_path(), #axum_path);
                            router = router.route(
                                &full_path,
                                ::chimera_web::prelude::axum::routing::#http_method(|
                                    ::chimera_web::prelude::axum::Extension(context):
                                        ::chimera_web::prelude::axum::Extension<::std::sync::Arc<::chimera_core::ApplicationContext>>,
                                    ::chimera_web::prelude::axum::extract::Path(#param_name):
                                        ::chimera_web::prelude::axum::extract::Path<String>,
                                | async move {
                                    // 从ApplicationContext获取controller bean
                                    match context.get_bean_by_type::<#self_ty>().await {
                                        Ok(controller) => {
                                            use ::chimera_web::prelude::IntoResponse;
                                            controller.#method_name(#param_name).await.into_response()
                                        }
                                        Err(_) => {
                                            use ::chimera_web::prelude::IntoResponse;
                                            (
                                                ::chimera_web::prelude::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                                "Controller bean not found"
                                            ).into_response()
                                        }
                                    }
                                })
                            );
                        }
                    }
                } else if !path_params.is_empty() && !fn_params.is_empty() {
                    // 多个路径参数的情况
                    let param_names = &fn_params[..path_params.len().min(fn_params.len())];
                    let param_tuple = quote! { (#(#param_names),*) };
                    let param_types: Vec<_> = param_names.iter().map(|_| quote! { String }).collect();
                    quote! {
                        // 拼接完整路径: base_path + method_path
                        let full_path = format!("{}{}", #self_ty::__base_path(), #path);
                        router = router.route(
                            &full_path,
                            ::chimera_web::prelude::axum::routing::#http_method(|
                                ::chimera_web::prelude::axum::Extension(context):
                                    ::chimera_web::prelude::axum::Extension<::std::sync::Arc<::chimera_core::ApplicationContext>>,
                                ::chimera_web::prelude::axum::extract::Path(#param_tuple):
                                    ::chimera_web::prelude::axum::extract::Path<(#(#param_types),*)>,
                            | async move {
                                // 从ApplicationContext获取controller bean
                                match context.get_bean_by_type::<#self_ty>().await {
                                    Ok(controller) => {
                                        use ::chimera_web::prelude::IntoResponse;
                                        controller.#method_name(#(#param_names),*).await.into_response()
                                    }
                                    Err(_) => {
                                        use ::chimera_web::prelude::IntoResponse;
                                        (
                                            ::chimera_web::prelude::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                            "Controller bean not found"
                                        ).into_response()
                                    }
                                }
                            })
                        );
                    }
                } else {
                    // 如果路径参数和函数参数不匹配，生成编译错误
                    quote! {
                        compile_error!("Path parameters do not match function parameters");
                    }
                };

                route_registrations.push(handler);
            }
        }
    }

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
        }
    };

    TokenStream::from(expanded)
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

/// 路径参数信息
#[derive(Debug, Clone)]
struct PathParam {
    name: String,
    pattern: Option<String>,  // 可选的正则表达式
}

/// 提取方法的路径参数
///
/// 支持以下语法：
/// - `/users/:id` - 简单参数
/// - `/users/:id<\d+>` - 带正则验证的参数
fn extract_path_params_from_path(path: &str) -> Vec<PathParam> {
    path.split('/')
        .filter(|seg| seg.starts_with(':'))
        .map(|seg| {
            let seg = &seg[1..]; // 移除前导的 ':'

            // 检查是否有正则表达式 :name<pattern>
            if let Some(angle_pos) = seg.find('<') {
                if seg.ends_with('>') {
                    let name = seg[..angle_pos].to_string();
                    let pattern = seg[angle_pos + 1..seg.len() - 1].to_string();
                    return PathParam {
                        name,
                        pattern: Some(pattern),
                    };
                }
            }

            // 简单参数，无正则
            PathParam {
                name: seg.to_string(),
                pattern: None,
            }
        })
        .collect()
}

/// 提取方法的函数参数名（跳过 &self）
fn extract_fn_params(method: &ImplItemFn) -> Vec<syn::Ident> {
    let mut params = Vec::new();

    for arg in &method.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            // 提取参数名
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                params.push(pat_ident.ident.clone());
            }
        }
    }

    params
}

/// 将路径转换为 Axum 格式
///
/// 移除正则表达式部分：`/users/:id<\d+>` -> `/users/:id`
fn convert_path_to_axum(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            if seg.starts_with(':') {
                if let Some(angle_pos) = seg.find('<') {
                    // 移除 <regex> 部分，只保留 :name
                    return &seg[..angle_pos];
                }
            }
            seg
        })
        .collect::<Vec<_>>()
        .join("/")
}

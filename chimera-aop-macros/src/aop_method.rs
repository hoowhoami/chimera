//! #[aop] 方法属性宏实现
//!
//! 用于自动为方法应用 AOP 切面

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, ItemFn, ReturnType, Signature};

/// 实现 #[aop] 属性宏
///
/// 为方法自动添加 AOP 功能，类似 Spring Boot 的声明式 AOP
pub fn impl_aop_method(target_type: Option<String>, method_name: Option<String>, mut item: ItemFn) -> TokenStream {
    let original_fn = item.clone();
    let fn_name = &item.sig.ident;
    let fn_vis = &item.vis;
    let fn_block = &item.block;
    let fn_inputs = &item.sig.inputs;
    let fn_output = &item.sig.output;
    let fn_generics = &item.sig.generics;
    let fn_asyncness = &item.sig.asyncness;

    // 提取目标类型和方法名
    let target = target_type.unwrap_or_else(|| "UnknownType".to_string());
    let method = method_name.unwrap_or_else(|| fn_name.to_string());

    // 检查是否是异步方法
    if fn_asyncness.is_none() {
        // 同步方法暂不支持
        return quote! {
            compile_error!("#[aop] currently only supports async methods");
        };
    }

    // 检查返回类型是否为 Result
    let is_result = match &fn_output {
        ReturnType::Type(_, ty) => {
            if let syn::Type::Path(type_path) = &**ty {
                type_path.path.segments.last()
                    .map(|seg| seg.ident == "Result")
                    .unwrap_or(false)
            } else {
                false
            }
        }
        _ => false,
    };

    if !is_result {
        return quote! {
            compile_error!("#[aop] currently only supports methods returning Result<T, E>");
        };
    }

    // 构建包装后的方法体
    let wrapped_block = quote! {
        {
            let jp = chimera_aop::JoinPoint::new(#target, #method);
            let registry = chimera_aop::get_global_registry();

            registry.execute_with_aspects(jp, || #fn_block).await
        }
    };

    // 替换方法体
    item.block = parse_quote!(#wrapped_block);

    quote! {
        #item
    }
}

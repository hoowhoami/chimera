//! Aspect 宏实现

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result, Error};

pub fn impl_aspect_derive(input: &DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // 查找 #[pointcut("...")] 属性
    let pointcut_expr = extract_pointcut_attr(input)?;

    // 生成 Aspect trait 实现
    let expanded = quote! {
        impl #impl_generics chimera_aop::Aspect for #name #ty_generics #where_clause {
            fn name(&self) -> &str {
                stringify!(#name)
            }

            fn pointcut(&self) -> &chimera_aop::PointcutExpression {
                use chimera_aop::PointcutExpression;

                // 创建静态的切点表达式
                static POINTCUT: once_cell::sync::Lazy<PointcutExpression> = once_cell::sync::Lazy::new(|| {
                    PointcutExpression::execution(#pointcut_expr)
                });

                &POINTCUT
            }
        }

        // 自动注册到 inventory
        // 使用 AspectRegistration 进行注册，类似于 Component 的 ComponentRegistry
        chimera_aop::inventory::submit! {
            chimera_aop::AspectRegistration::new(
                stringify!(#name),
                #pointcut_expr,
                || std::sync::Arc::new(#name::new()) as std::sync::Arc<dyn chimera_aop::Aspect>
            )
        }
    };

    Ok(expanded)
}

fn extract_pointcut_attr(input: &DeriveInput) -> Result<String> {
    for attr in &input.attrs {
        if attr.path().is_ident("pointcut") {
            // 解析 #[pointcut("expression")]
            let expr: syn::LitStr = attr.parse_args()?;
            return Ok(expr.value());
        }
    }

    Err(Error::new_spanned(
        input,
        "#[derive(Aspect)] requires #[pointcut(\"expression\")] attribute"
    ))
}

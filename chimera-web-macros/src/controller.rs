//! Controller 宏实现

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

use crate::utils;

/// Controller 宏实现
pub fn derive_controller_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // 提取 request_mapping 属性
    let base_path = utils::extract_request_mapping(&input.attrs);

    // 生成提交到全局注册表的代码
    let expanded = quote! {
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
                register: |router, context| {
                    #name::__register_routes(router, context)
                },
            }
        }
    };

    TokenStream::from(expanded)
}

//! 工具函数

use proc_macro2::TokenStream;
use quote::ToTokens;

/// 将错误转换为编译错误
pub fn to_compile_error(err: syn::Error) -> TokenStream {
    err.to_compile_error()
}

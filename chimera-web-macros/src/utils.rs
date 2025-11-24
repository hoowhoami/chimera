//! 宏辅助工具函数

use syn::{Attribute, Meta};

/// 从属性中提取 route 的值
pub fn extract_request_mapping(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident("route"))
        .and_then(|attr| {
            if let Meta::List(meta_list) = &attr.meta {
                // 解析 #[route("/path")]
                meta_list.tokens.clone().into_iter().next().and_then(|token| {
                    if let proc_macro2::TokenTree::Literal(lit) = token {
                        let lit_str = lit.to_string();
                        // 移除引号
                        Some(lit_str.trim_matches('"').to_string())
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .unwrap_or_else(|| String::from("/"))
}

/// 从属性中提取字符串字面量
#[allow(dead_code)]
pub fn extract_string_literal(attr: &Attribute) -> Option<String> {
    if let Meta::List(meta_list) = &attr.meta {
        meta_list.tokens.clone().into_iter().next().and_then(|token| {
            if let proc_macro2::TokenTree::Literal(lit) = token {
                let lit_str = lit.to_string();
                Some(lit_str.trim_matches('"').to_string())
            } else {
                None
            }
        })
    } else {
        None
    }
}

//! 宏辅助工具函数

use syn::{Attribute, Meta};

/// 从属性中提取 request_mapping 的值
pub fn extract_request_mapping(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident("request_mapping"))
        .and_then(|attr| {
            if let Meta::List(meta_list) = &attr.meta {
                // 解析 #[request_mapping("/path")]
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

/// 检查属性是否匹配指定名称
#[allow(dead_code)]
pub fn is_attribute(attr: &Attribute, name: &str) -> bool {
    attr.path().is_ident(name)
}

/// 解析方法参数的注解
#[allow(dead_code)]
pub fn parse_param_annotations() {
    // TODO: 实现参数注解解析
    // - #[path_param] 或 #[path_param("name")]
    // - #[query_param] 或 #[query_param("name")]
    // - #[request_body]
    // - #[header("name")]
}

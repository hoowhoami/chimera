use syn::{Attribute, Expr, Meta};
use quote::quote;

/// Value字段信息
pub(crate) struct ValueFieldInfo {
    pub key: String,
    pub default_value: Option<proc_macro2::TokenStream>,
}

/// 从属性中提取value配置
///
/// 支持格式：
/// - `#[value("config.key")]` - 必需配置
/// - `#[value("config.key", default = value)]` - 可选配置，带默认值
pub(crate) fn get_value_info(attrs: &[Attribute]) -> Option<ValueFieldInfo> {
    for attr in attrs {
        if attr.path().is_ident("value") {
            // 解析 #[value("key")] 或 #[value("key", default = "value")]
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = &meta_list.tokens;
                let tokens_str = tokens.to_string();

                // 简单解析：分割逗号
                let parts: Vec<&str> = tokens_str.split(',').map(|s| s.trim()).collect();

                if parts.is_empty() {
                    continue;
                }

                // 第一部分是key（去掉引号）
                let key = parts[0].trim_matches('"').to_string();

                // 查找default参数
                let mut default_value = None;
                for part in &parts[1..] {
                    if part.contains("default") {
                        // 解析 default = value
                        if let Some(eq_pos) = part.find('=') {
                            let value_str = part[eq_pos + 1..].trim();
                            // 将字符串解析为TokenStream
                            if let Ok(expr) = syn::parse_str::<Expr>(value_str) {
                                default_value = Some(quote! { #expr });
                            }
                        }
                    }
                }

                return Some(ValueFieldInfo { key, default_value });
            }
        }
    }
    None
}

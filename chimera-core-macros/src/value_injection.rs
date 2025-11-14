use syn::{Attribute, Expr, Token};
use syn::parse::{Parse, ParseStream};
use quote::quote;

/// Value字段信息
pub(crate) struct ValueFieldInfo {
    pub key: String,
    pub default_value: Option<proc_macro2::TokenStream>,
}

/// 解析 #[value] 属性的参数
struct ValueArgs {
    key: syn::LitStr,
    default_value: Option<Expr>,
}

impl Parse for ValueArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // 解析第一个参数：key（字符串字面量）
        let key: syn::LitStr = input.parse()?;

        let mut default_value = None;

        // 检查是否有逗号和更多参数
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;

            // 解析 default = value
            if input.peek(syn::Ident) {
                let ident: syn::Ident = input.parse()?;
                if ident == "default" {
                    input.parse::<Token![=]>()?;
                    let expr: Expr = input.parse()?;
                    default_value = Some(expr);
                }
            }
        }

        Ok(ValueArgs { key, default_value })
    }
}

/// 从属性中提取value配置
///
/// 支持格式：
/// - `#[value("config.key")]` - 必需配置
/// - `#[value("config.key", default = value)]` - 可选配置，带默认值
pub(crate) fn get_value_info(attrs: &[Attribute]) -> Option<ValueFieldInfo> {
    for attr in attrs {
        if attr.path().is_ident("value") {
            // 使用 syn 的正式解析器来解析属性
            if let Ok(args) = attr.parse_args::<ValueArgs>() {
                let key = args.key.value();
                let default_value = args.default_value.map(|expr| quote! { #expr });

                return Some(ValueFieldInfo { key, default_value });
            }
        }
    }
    None
}

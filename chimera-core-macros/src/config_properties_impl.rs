use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Attribute};

/// ConfigurationProperties 配置信息
struct ConfigPropertiesInfo {
    prefix: Option<String>,
}

/// 从属性中提取 ConfigurationProperties 配置
///
/// 支持格式：
/// - `#[prefix("database")]` - 指定配置前缀
fn get_config_properties_info(attrs: &[Attribute]) -> Option<ConfigPropertiesInfo> {
    for attr in attrs {
        if attr.path().is_ident("prefix") {
            if let Ok(prefix_lit) = attr.parse_args::<syn::LitStr>() {
                return Some(ConfigPropertiesInfo {
                    prefix: Some(prefix_lit.value()),
                });
            }
        }
    }
    None
}

/// 字段配置信息
struct FieldConfigInfo {
    config_key: Option<String>,
}

/// 从字段属性中提取配置信息
///
/// 支持格式：
/// - `#[config("custom-key")]` - 自定义配置键名
fn get_field_config_info(attrs: &[Attribute]) -> Option<FieldConfigInfo> {
    for attr in attrs {
        if attr.path().is_ident("config") {
            if let Ok(key_lit) = attr.parse_args::<syn::LitStr>() {
                return Some(FieldConfigInfo {
                    config_key: Some(key_lit.value()),
                });
            }
        }
    }
    None
}

/// 将字段名转换为 kebab-case 配置键
///
/// 例如：max_size -> max-size
fn field_name_to_config_key(field_name: &str) -> String {
    field_name.replace('_', "-")
}

pub(crate) fn derive_configuration_properties_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    // 获取前缀配置
    let prefix = get_config_properties_info(&input.attrs)
        .and_then(|info| info.prefix)
        .unwrap_or_default();

    // 提取所有字段
    let fields = if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields) = &data_struct.fields {
            fields.named.iter().collect::<Vec<_>>()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // 生成字段绑定代码
    let field_bindings = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;

        // 获取字段的配置键名
        let config_key = if let Some(field_config) = get_field_config_info(&field.attrs) {
            field_config.config_key.unwrap_or_else(|| {
                field_name_to_config_key(&field_name.as_ref().unwrap().to_string())
            })
        } else {
            field_name_to_config_key(&field_name.as_ref().unwrap().to_string())
        };

        // 完整的配置路径
        let full_key = if prefix.is_empty() {
            config_key.clone()
        } else {
            format!("{}.{}", prefix, config_key)
        };

        // 根据类型生成绑定代码
        let type_str = quote! { #field_type }.to_string();

        // 检查是否是 Option 类型
        let is_option = type_str.contains("Option");

        if is_option {
            // Option 类型：可选配置
            if type_str.contains("String") {
                quote! {
                    let #field_name = env.get_string(#full_key);
                }
            } else if type_str.contains("i64") || type_str.contains("i32") || type_str.contains("u64") || type_str.contains("u32") {
                quote! {
                    let #field_name = env.get_i64(#full_key).map(|v| v as _);
                }
            } else if type_str.contains("f64") || type_str.contains("f32") {
                quote! {
                    let #field_name = env.get_f64(#full_key).map(|v| v as _);
                }
            } else if type_str.contains("bool") {
                quote! {
                    let #field_name = env.get_bool(#full_key);
                }
            } else {
                quote! {
                    let #field_name = env.get_string(#full_key)
                        .and_then(|s| s.parse().ok());
                }
            }
        } else {
            // 必需配置
            if type_str.contains("String") {
                quote! {
                    let #field_name = env.get_string(#full_key)
                        .ok_or_else(|| anyhow::anyhow!("Required config '{}' not found", #full_key))?;
                }
            } else if type_str.contains("i64") || type_str.contains("i32") || type_str.contains("u64") || type_str.contains("u32") {
                quote! {
                    let #field_name = env.get_i64(#full_key)
                        .ok_or_else(|| anyhow::anyhow!("Required config '{}' not found", #full_key))? as #field_type;
                }
            } else if type_str.contains("f64") || type_str.contains("f32") {
                quote! {
                    let #field_name = env.get_f64(#full_key)
                        .ok_or_else(|| anyhow::anyhow!("Required config '{}' not found", #full_key))? as #field_type;
                }
            } else if type_str.contains("bool") {
                quote! {
                    let #field_name = env.get_bool(#full_key)
                        .ok_or_else(|| anyhow::anyhow!("Required config '{}' not found", #full_key))?;
                }
            } else {
                // 其他类型，尝试从字符串解析
                quote! {
                    let #field_name = env.get_string(#full_key)
                        .ok_or_else(|| anyhow::anyhow!("Required config '{}' not found", #full_key))?
                        .parse::<#field_type>()
                        .map_err(|e| anyhow::anyhow!("Failed to parse config '{}': {:?}", #full_key, e))?;
                }
            }
        }
    });

    let field_names: Vec<_> = fields.iter()
        .map(|f| &f.ident)
        .collect();

    // 生成 bean 名称（使用类型名的 camelCase 形式）
    let bean_name = {
        let name_str = name.to_string();
        let mut chars = name_str.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        }
    };

    // 生成实现代码
    let expanded = quote! {
        impl #name {
            /// 从 Environment 绑定配置
            pub fn bind(env: &std::sync::Arc<chimera_core::config::Environment>) -> std::result::Result<Self, anyhow::Error> {
                #(#field_bindings)*

                Ok(Self {
                    #(#field_names),*
                })
            }

            /// 注册到容器（内部使用）
            fn __register_to_context(context: &std::sync::Arc<chimera_core::ApplicationContext>) -> chimera_core::ContainerResult<()> {
                let env = std::sync::Arc::clone(context.environment());

                // 绑定配置
                let instance = Self::bind(&env)
                    .map_err(|e| chimera_core::ContainerError::BeanCreationFailed(
                        format!("{}: {}", #bean_name, e)
                    ))?;

                // 注册为单例 Bean
                context.register_singleton(#bean_name, move || {
                    Ok(instance.clone())
                })?;

                Ok(())
            }
        }

        // 自动向inventory注册
        inventory::submit! {
            chimera_core::component::ConfigurationPropertiesRegistry {
                registrar: |ctx: &std::sync::Arc<chimera_core::ApplicationContext>| {
                    #name::__register_to_context(ctx)
                },
                name: #bean_name,
            }
        }
    };

    TokenStream::from(expanded)
}

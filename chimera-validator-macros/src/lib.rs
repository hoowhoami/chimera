use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Expr, ExprLit, Lit, Token, Type};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

// 检测类型是否是 Option<T>
fn is_option(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

// 定���验证参数
struct ValidationArgs {
    args: Punctuated<ValidationArg, Token![,]>,
}

struct ValidationArg {
    name: syn::Ident,
    value: syn::Expr,
}

impl Parse for ValidationArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ValidationArgs {
            args: input.parse_terminated(ValidationArg::parse, Token![,])?,
        })
    }
}

impl Parse for ValidationArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: syn::Expr = input.parse()?;
        Ok(ValidationArg { name, value })
    }
}

#[proc_macro_derive(Validate, attributes(validate))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let validations = match &input.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => {
                    let field_validations: Vec<_> = fields.named.iter().map(|field| {
                        let field_name = field.ident.as_ref().unwrap();
                        let field_name_str = field_name.to_string();
                        let mut validations = Vec::new();

                        // 检测字段是否是 Option 类型
                        let is_option_type = is_option(&field.ty);

                        for attr in &field.attrs {
                            if attr.path().is_ident("validate") {
                                let _ = attr.parse_nested_meta(|meta| {
                                    let path = meta.path.get_ident().unwrap().to_string();

                                    match path.as_str() {
                                        "not_blank" => {
                                            let mut custom_message: Option<String> = None;

                                            // 检查是否有参数（如 message）
                                            if meta.input.peek(syn::token::Paren) {
                                                let content;
                                                syn::parenthesized!(content in meta.input);
                                                if let Ok(args) = content.parse::<ValidationArgs>() {
                                                    for arg in args.args {
                                                        if arg.name == "message" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = arg.value {
                                                                custom_message = Some(lit_str.value());
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            if is_option_type {
                                                // Option<T> 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        if let Some(ref __val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::not_blank_with_message(
                                                                    __val,
                                                                    #field_name_str,
                                                                    Some(#msg)
                                                                )
                                                            );
                                                        }
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        if let Some(ref __val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::not_blank(
                                                                    __val,
                                                                    #field_name_str
                                                                )
                                                            );
                                                        }
                                                    });
                                                }
                                            } else {
                                                // 非 Option 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::not_blank_with_message(
                                                                &self.#field_name,
                                                                #field_name_str,
                                                                Some(#msg)
                                                            )
                                                        );
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::not_blank(
                                                                &self.#field_name,
                                                                #field_name_str
                                                            )
                                                        );
                                                    });
                                                }
                                            }
                                        }
                                        "not_empty" => {
                                            let mut custom_message: Option<String> = None;

                                            if meta.input.peek(syn::token::Paren) {
                                                let content;
                                                syn::parenthesized!(content in meta.input);
                                                if let Ok(args) = content.parse::<ValidationArgs>() {
                                                    for arg in args.args {
                                                        if arg.name == "message" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = arg.value {
                                                                custom_message = Some(lit_str.value());
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            if is_option_type {
                                                // Option<T> 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        if let Some(ref __val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::not_empty_with_message(
                                                                    __val,
                                                                    #field_name_str,
                                                                    Some(#msg)
                                                                )
                                                            );
                                                        }
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        if let Some(ref __val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::not_empty(
                                                                    __val,
                                                                    #field_name_str
                                                                )
                                                            );
                                                        }
                                                    });
                                                }
                                            } else {
                                                // 非 Option 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::not_empty_with_message(
                                                                &self.#field_name,
                                                                #field_name_str,
                                                                Some(#msg)
                                                            )
                                                        );
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::not_empty(
                                                                &self.#field_name,
                                                                #field_name_str
                                                            )
                                                        );
                                                    });
                                                }
                                            }
                                        }
                                        "email" => {
                                            let mut custom_message: Option<String> = None;

                                            if meta.input.peek(syn::token::Paren) {
                                                let content;
                                                syn::parenthesized!(content in meta.input);
                                                if let Ok(args) = content.parse::<ValidationArgs>() {
                                                    for arg in args.args {
                                                        if arg.name == "message" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = arg.value {
                                                                custom_message = Some(lit_str.value());
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            if is_option_type {
                                                // Option<T> 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        if let Some(ref __val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::email_with_message(
                                                                    __val,
                                                                    #field_name_str,
                                                                    Some(#msg)
                                                                )
                                                            );
                                                        }
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        if let Some(ref __val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::email(
                                                                    __val,
                                                                    #field_name_str
                                                                )
                                                            );
                                                        }
                                                    });
                                                }
                                            } else {
                                                // 非 Option 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::email_with_message(
                                                                &self.#field_name,
                                                                #field_name_str,
                                                                Some(#msg)
                                                            )
                                                        );
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::email(
                                                                &self.#field_name,
                                                                #field_name_str
                                                            )
                                                        );
                                                    });
                                                }
                                            }
                                        }
                                        "length" => {
                                            let mut min_val: Option<usize> = None;
                                            let mut max_val: Option<usize> = None;
                                            let mut custom_message: Option<String> = None;

                                            // 解析参数
                                            if meta.input.peek(syn::token::Paren) {
                                                let content;
                                                syn::parenthesized!(content in meta.input);
                                                if let Ok(args) = content.parse::<ValidationArgs>() {
                                                    for arg in args.args {
                                                        if arg.name == "min" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = arg.value {
                                                                min_val = lit_int.base10_parse().ok();
                                                            }
                                                        } else if arg.name == "max" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = arg.value {
                                                                max_val = lit_int.base10_parse().ok();
                                                            }
                                                        } else if arg.name == "message" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = arg.value {
                                                                custom_message = Some(lit_str.value());
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            let min_opt = if let Some(m) = min_val {
                                                quote! { Some(#m) }
                                            } else {
                                                quote! { None }
                                            };

                                            let max_opt = if let Some(m) = max_val {
                                                quote! { Some(#m) }
                                            } else {
                                                quote! { None }
                                            };

                                            if is_option_type {
                                                // Option<T> 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        if let Some(ref __val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::length_with_message(
                                                                    __val,
                                                                    #field_name_str,
                                                                    #min_opt,
                                                                    #max_opt,
                                                                    Some(#msg)
                                                                )
                                                            );
                                                        }
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        if let Some(ref __val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::length(
                                                                    __val,
                                                                    #field_name_str,
                                                                    #min_opt,
                                                                    #max_opt
                                                                )
                                                            );
                                                        }
                                                    });
                                                }
                                            } else {
                                                // 非 Option 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::length_with_message(
                                                                &self.#field_name,
                                                                #field_name_str,
                                                                #min_opt,
                                                                #max_opt,
                                                                Some(#msg)
                                                            )
                                                        );
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::length(
                                                                &self.#field_name,
                                                                #field_name_str,
                                                                #min_opt,
                                                                #max_opt
                                                            )
                                                        );
                                                    });
                                                }
                                            }
                                        }
                                        "length_min" => {
                                            let value: Expr = meta.value()?.parse()?;
                                            if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = value {
                                                let min: usize = lit_int.base10_parse()?;
                                                validations.push(quote! {
                                                    validator.add_result(
                                                        chimera_validator::ValidationRules::length(
                                                            &self.#field_name,
                                                            #field_name_str,
                                                            Some(#min),
                                                            None
                                                        )
                                                    );
                                                });
                                            }
                                        }
                                        "length_max" => {
                                            let value: Expr = meta.value()?.parse()?;
                                            if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = value {
                                                let max: usize = lit_int.base10_parse()?;
                                                validations.push(quote! {
                                                    validator.add_result(
                                                        chimera_validator::ValidationRules::length(
                                                            &self.#field_name,
                                                            #field_name_str,
                                                            None,
                                                            Some(#max)
                                                        )
                                                    );
                                                });
                                            }
                                        }
                                        "range" => {
                                            let mut min_val: Option<String> = None;
                                            let mut max_val: Option<String> = None;
                                            let mut custom_message: Option<String> = None;

                                            if meta.input.peek(syn::token::Paren) {
                                                let content;
                                                syn::parenthesized!(content in meta.input);
                                                if let Ok(args) = content.parse::<ValidationArgs>() {
                                                    for arg in args.args {
                                                        if arg.name == "min" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = arg.value {
                                                                min_val = Some(lit_int.base10_digits().to_string());
                                                            }
                                                        } else if arg.name == "max" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = arg.value {
                                                                max_val = Some(lit_int.base10_digits().to_string());
                                                            }
                                                        } else if arg.name == "message" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = arg.value {
                                                                custom_message = Some(lit_str.value());
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            let min_opt = if let Some(m) = min_val {
                                                let m_token: proc_macro2::TokenStream = m.parse().unwrap();
                                                quote! { Some(#m_token) }
                                            } else {
                                                quote! { None }
                                            };

                                            let max_opt = if let Some(m) = max_val {
                                                let m_token: proc_macro2::TokenStream = m.parse().unwrap();
                                                quote! { Some(#m_token) }
                                            } else {
                                                quote! { None }
                                            };

                                            if is_option_type {
                                                // Option<T> 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        if let Some(__val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::range_with_message(
                                                                    __val,
                                                                    #field_name_str,
                                                                    #min_opt,
                                                                    #max_opt,
                                                                    Some(#msg)
                                                                )
                                                            );
                                                        }
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        if let Some(__val) = self.#field_name {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::range(
                                                                    __val,
                                                                    #field_name_str,
                                                                    #min_opt,
                                                                    #max_opt
                                                                )
                                                            );
                                                        }
                                                    });
                                                }
                                            } else {
                                                // 非 Option 类型
                                                if let Some(msg) = custom_message {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::range_with_message(
                                                                self.#field_name,
                                                                #field_name_str,
                                                                #min_opt,
                                                                #max_opt,
                                                                Some(#msg)
                                                            )
                                                        );
                                                    });
                                                } else {
                                                    validations.push(quote! {
                                                        validator.add_result(
                                                            chimera_validator::ValidationRules::range(
                                                                self.#field_name,
                                                                #field_name_str,
                                                                #min_opt,
                                                                #max_opt
                                                            )
                                                        );
                                                    });
                                                }
                                            }
                                        }
                                        "min" => {
                                            let value: Expr = meta.value()?.parse()?;
                                            if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = value {
                                                let min = lit_int.base10_digits();
                                                let min_token: proc_macro2::TokenStream = min.parse().unwrap();
                                                validations.push(quote! {
                                                    validator.add_result(
                                                        chimera_validator::ValidationRules::range(
                                                            self.#field_name,
                                                            #field_name_str,
                                                            Some(#min_token),
                                                            None
                                                        )
                                                    );
                                                });
                                            }
                                        }
                                        "max" => {
                                            let value: Expr = meta.value()?.parse()?;
                                            if let Expr::Lit(ExprLit { lit: Lit::Int(lit_int), .. }) = value {
                                                let max = lit_int.base10_digits();
                                                let max_token: proc_macro2::TokenStream = max.parse().unwrap();
                                                validations.push(quote! {
                                                    validator.add_result(
                                                        chimera_validator::ValidationRules::range(
                                                            self.#field_name,
                                                            #field_name_str,
                                                            None,
                                                            Some(#max_token)
                                                        )
                                                    );
                                                });
                                            }
                                        }
                                        "pattern" => {
                                            let mut pattern_str: Option<String> = None;
                                            let mut custom_message: Option<String> = None;

                                            // 尝试解析两种语法：
                                            // 1. pattern = "regex" (旧语法，不支持 message)
                                            // 2. pattern(regex = "regex", message = "msg") (新语法)
                                            if meta.input.peek(syn::token::Paren) {
                                                // 新语法：pattern(regex = "...", message = "...")
                                                let content;
                                                syn::parenthesized!(content in meta.input);
                                                if let Ok(args) = content.parse::<ValidationArgs>() {
                                                    for arg in args.args {
                                                        if arg.name == "regex" || arg.name == "pattern" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = arg.value {
                                                                pattern_str = Some(lit_str.value());
                                                            }
                                                        } else if arg.name == "message" {
                                                            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = arg.value {
                                                                custom_message = Some(lit_str.value());
                                                            }
                                                        }
                                                    }
                                                }
                                            } else if let Ok(value) = meta.value() {
                                                // 旧语法：pattern = "regex"
                                                let expr: Expr = value.parse()?;
                                                if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = expr {
                                                    pattern_str = Some(lit_str.value());
                                                }
                                            }

                                            if let Some(pattern) = pattern_str {
                                                // 根据字段类型生成不同的验证代码
                                                if is_option_type {
                                                    // Option<T> 类型：只验证存在的值
                                                    if let Some(msg) = custom_message {
                                                        validations.push(quote! {
                                                            if let Some(ref __val) = self.#field_name {
                                                                validator.add_result(
                                                                    chimera_validator::ValidationRules::pattern_with_message(
                                                                        __val,
                                                                        #field_name_str,
                                                                        #pattern,
                                                                        Some(#msg)
                                                                    )
                                                                );
                                                            }
                                                        });
                                                    } else {
                                                        validations.push(quote! {
                                                            if let Some(ref __val) = self.#field_name {
                                                                validator.add_result(
                                                                    chimera_validator::ValidationRules::pattern(
                                                                        __val,
                                                                        #field_name_str,
                                                                        #pattern
                                                                    )
                                                                );
                                                            }
                                                        });
                                                    }
                                                } else {
                                                    // 非 Option 类型
                                                    if let Some(msg) = custom_message {
                                                        validations.push(quote! {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::pattern_with_message(
                                                                    &self.#field_name,
                                                                    #field_name_str,
                                                                    #pattern,
                                                                    Some(#msg)
                                                                )
                                                            );
                                                        });
                                                    } else {
                                                        validations.push(quote! {
                                                            validator.add_result(
                                                                chimera_validator::ValidationRules::pattern(
                                                                    &self.#field_name,
                                                                    #field_name_str,
                                                                    #pattern
                                                                )
                                                            );
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }

                                    Ok(())
                                });
                            }
                        }

                        validations
                    }).flatten().collect();

                    field_validations
                }
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    };

    let expanded = quote! {
        impl chimera_validator::Validate for #name {
            fn validate(&self) -> chimera_validator::ValidationResult<()> {
                let mut validator = chimera_validator::ValidatorBuilder::new();

                #(#validations)*

                validator.build()
            }
        }
    };

    TokenStream::from(expanded)
}

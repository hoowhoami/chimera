use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Expr, ExprLit, Lit, ItemFn, FnArg, Pat, Type};

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

                        for attr in &field.attrs {
                            if attr.path().is_ident("validate") {
                                let _ = attr.parse_nested_meta(|meta| {
                                    let path = meta.path.get_ident().unwrap().to_string();

                                    match path.as_str() {
                                        "not_blank" => {
                                            validations.push(quote! {
                                                validator.add_result(
                                                    chimera_validator::ValidationRules::not_blank(
                                                        &self.#field_name,
                                                        #field_name_str
                                                    )
                                                );
                                            });
                                        }
                                        "not_empty" => {
                                            validations.push(quote! {
                                                validator.add_result(
                                                    chimera_validator::ValidationRules::not_empty(
                                                        &self.#field_name,
                                                        #field_name_str
                                                    )
                                                );
                                            });
                                        }
                                        "email" => {
                                            validations.push(quote! {
                                                validator.add_result(
                                                    chimera_validator::ValidationRules::email(
                                                        &self.#field_name,
                                                        #field_name_str
                                                    )
                                                );
                                            });
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
                                            let value: Expr = meta.value()?.parse()?;
                                            if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = value {
                                                let pattern = lit_str.value();
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

/// `#[valid]` 宏 - 自动验证方法参数
///
/// 用于 controller 方法，自动验证实现了 Validate trait 的参数
///
/// # 示例
///
/// ```rust
/// #[post_mapping("/users")]
/// #[valid]
/// async fn create_user(
///     RequestBody(request): RequestBody<CreateUserRequest>
/// ) -> impl IntoResponse {
///     // request 已经通过验证
/// }
/// ```
#[proc_macro_attribute]
pub fn valid(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let _fn_name = &input.sig.ident;
    let _fn_vis = &input.vis;
    let fn_sig = &input.sig;
    let fn_block = &input.block;
    let fn_attrs = &input.attrs;

    // 提取方法参数并生成验证语句
    let mut validation_stmts = Vec::new();

    for arg in input.sig.inputs.iter() {
        if let FnArg::Typed(pat_type) = arg {
            // 检查参数类型是否是 RequestBody<T>
            if let Type::Path(type_path) = &*pat_type.ty {
                let type_str = quote!(#type_path).to_string();

                // 如果是 RequestBody 类型，提取内部值并验证
                if type_str.contains("RequestBody") {
                    // 匹配 RequestBody(inner) 模式
                    if let Pat::TupleStruct(tuple_struct) = &*pat_type.pat {
                        if let Some(Pat::Ident(inner_ident)) = tuple_struct.elems.first() {
                            let inner_name = &inner_ident.ident;
                            validation_stmts.push(quote! {
                                if let Err(e) = chimera_validator::Validate::validate(&#inner_name) {
                                    use chimera_web::prelude::*;
                                    return chimera_web::exception_handler::ApplicationError::ValidationError(
                                        format!("{:?}", e)
                                    ).into_response();
                                }
                            });
                        }
                    }
                }
            }
        }
    }

    // 生成新的函数
    let expanded = quote! {
        #(#fn_attrs)*
        #fn_sig {
            // 验证参数
            #(#validation_stmts)*

            // 执行原方法体
            #fn_block
        }
    };

    TokenStream::from(expanded)
}


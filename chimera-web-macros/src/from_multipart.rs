use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

pub fn derive_from_multipart_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("FromMultipart only supports structs with named fields"),
        },
        _ => panic!("FromMultipart can only be derived for structs"),
    };

    let mut field_extractions = Vec::new();
    let mut field_names = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_type = &field.ty;

        field_names.push(field_name);

        // 判断字段类型
        if is_multipart_file_type(field_type) {
            // MultipartFile
            field_extractions.push(quote! {
                let #field_name = raw.get_file(#field_name_str)
                    .ok_or_else(|| chimera_web::exception_handler::WebError::FormParse {
                        message: format!("Missing required file field: {}", #field_name_str),
                    })?;
            });
        } else if is_option_multipart_file_type(field_type) {
            // Option<MultipartFile>
            field_extractions.push(quote! {
                let #field_name = raw.get_file(#field_name_str);
            });
        } else if is_vec_multipart_file_type(field_type) {
            // Vec<MultipartFile>
            field_extractions.push(quote! {
                let #field_name = raw.get_files(#field_name_str);
            });
        } else if is_option_string_type(field_type) {
            // Option<String>
            field_extractions.push(quote! {
                let #field_name = raw.fields.get(#field_name_str).cloned();
            });
        } else {
            // 其他类型（String, i32, bool 等）需要解析
            field_extractions.push(quote! {
                let #field_name = raw.fields
                    .get(#field_name_str)
                    .ok_or_else(|| chimera_web::exception_handler::WebError::FormParse {
                        message: format!("Missing required field: {}", #field_name_str),
                    })?
                    .parse()
                    .map_err(|_| chimera_web::exception_handler::WebError::FormParse {
                        message: format!("Failed to parse field '{}' as {}", #field_name_str, stringify!(#field_type)),
                    })?;
            });
        }
    }

    let expanded = quote! {
        impl chimera_web::multipart::FromMultipart for #name {
            fn from_multipart(mut raw: chimera_web::multipart::MultipartRawData) -> Result<Self, chimera_web::exception_handler::WebError> {
                #(#field_extractions)*

                Ok(Self {
                    #(#field_names),*
                })
            }
        }
    };

    TokenStream::from(expanded)
}

fn is_multipart_file_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "MultipartFile";
        }
    }
    false
}

fn is_option_multipart_file_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return is_multipart_file_type(inner_ty);
                    }
                }
            }
        }
    }
    false
}

fn is_vec_multipart_file_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return is_multipart_file_type(inner_ty);
                    }
                }
            }
        }
    }
    false
}

fn is_option_string_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(Type::Path(inner_path))) = args.args.first() {
                        if let Some(inner_segment) = inner_path.path.segments.last() {
                            return inner_segment.ident == "String";
                        }
                    }
                }
            }
        }
    }
    false
}

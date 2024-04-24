//! `FromRhai` derive macro.

use proc_macro2::TokenStream;
use quote::quote;

use crate::VitrineAttribute;

/// Expand `FromRhai` derive macro.
pub fn expand_from_rhai(input: &syn::DeriveInput) -> TokenStream {
    let crate_path = quote! { ::vitrine::util::eval::rhai };

    let struct_ident = &input.ident;

    let syn::Data::Struct(data) = &input.data else {
        return syn::Error::new(struct_ident.span(), "Only structs can derive `FromRhai`")
            .to_compile_error();
    };

    let syn::Fields::Named(fields) = &data.fields else {
        return syn::Error::new(
            struct_ident.span(),
            "Only named structs can derive `FromRhai`",
        )
        .to_compile_error();
    };

    let fields =
        fields.named.iter().map(|field| {
            let field_ident = field.ident.as_ref().unwrap();

            let err_with_field = quote! {
                |source| #crate_path::RhaiError::WithField {
                    source: ::std::boxed::Box::new(source),
                    field: ::std::stringify!(#field_ident).to_string(),
                }
            };

            field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("vitrine"))
            .map(|attr| attr.parse_args())
            .next()
            .transpose()
            .unwrap()
            .map_or_else(
                || {
                    // No attribute
                    quote! {
                        #field_ident: #crate_path::FromRhai::from_rhai(
                            map.remove(::std::stringify!(#field_ident)).ok_or_else(
                                || #crate_path::RhaiError::FromRhai {
                                    from: type_name,
                                    to: ::std::stringify!(#struct_ident),
                                    message: ::std::option::Option::Some(::std::format!(
                                        "required field `{}`",
                                        ::std::stringify!(#field_ident),
                                    )),
                                }
                            )?,
                            runtime,
                        ).map_err(#err_with_field)?
                    }
                },|attr| match attr {
                VitrineAttribute::Default(path) => match path {
                    // `vitrine(default = "path")`
                    Some(path) => quote! {
                        #field_ident: map.remove(::std::stringify!(#field_ident)).map_or_else(
                            || ::std::result::Result::Ok(#path()),
                            |v| #crate_path::FromRhai::from_rhai(v, runtime),
                        ).map_err(#err_with_field)?
                    },
                    // `vitrine(default)`
                    None => quote! {
                        #field_ident: map.remove(::std::stringify!(#field_ident)).map_or_else(
                            || ::std::result::Result::Ok(::std::default::Default::default()),
                            |v| #crate_path::FromRhai::from_rhai(v, runtime),
                        ).map_err(#err_with_field)?
                    },
                },
                VitrineAttribute::Skip => {
                    // vitrine(skip)
                    quote! {
                        #field_ident: ::std::default::Default::default()
                    }
                },
            })
        });

    quote! {
        impl #crate_path::FromRhai for #struct_ident {
            fn from_rhai(
                value: ::rhai::Dynamic,
                runtime: &::std::sync::Arc<(::rhai::Engine, ::rhai::AST)>,
            ) -> ::std::result::Result<Self, #crate_path::RhaiError> {
                let type_name = value.type_name();
                let mut map =
                    value
                        .try_cast::<::rhai::Map>()
                        .ok_or_else(|| #crate_path::RhaiError::FromRhai {
                            from: type_name,
                            to: ::std::stringify!(#struct_ident),
                            message: ::std::option::Option::Some("expected map".to_string()),
                        })?;

                ::std::result::Result::Ok(Self {
                    #(#fields),*
                })
            }
        }
    }
}

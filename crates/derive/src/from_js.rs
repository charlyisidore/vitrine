//! `FromJs` derive macro.

use proc_macro2::TokenStream;
use quote::quote;

use crate::VitrineAttribute;

/// Expand `FromJs` derive macro.
pub fn expand_from_js(input: &syn::DeriveInput) -> TokenStream {
    let crate_path = quote! { ::vitrine::util::eval::js };

    let struct_ident = &input.ident;

    let syn::Data::Struct(data) = &input.data else {
        return syn::Error::new(struct_ident.span(), "Only structs can derive `FromJs`")
            .to_compile_error();
    };

    let syn::Fields::Named(fields) = &data.fields else {
        return syn::Error::new(
            struct_ident.span(),
            "Only named structs can derive `FromJs`",
        )
        .to_compile_error();
    };

    let fields = fields.named.iter().map(|field| {
        let field_ident = field.ident.as_ref().unwrap();

        let err_with_field = quote! {
            |source| #crate_path::JsError::WithField {
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
                        #field_ident: #crate_path::FromJs::from_js(
                            object.remove(::std::stringify!(#field_ident)).ok_or_else(
                                || #crate_path::JsError::FromJs {
                                    from: value.get_value_type().to_string(),
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
                },
                |attr| match attr {
                VitrineAttribute::Default(path) => match path {
                    // `vitrine(default = "path")`
                    Some(path) => quote! {
                        #field_ident: object.remove(::std::stringify!(#field_ident)).map_or_else(
                            || ::std::result::Result::Ok(#path()),
                            |v| #crate_path::FromJs::from_js(v, runtime),
                        ).map_err(#err_with_field)?
                    },
                    // `vitrine(default)`
                    None => quote! {
                        #field_ident: object.remove(::std::stringify!(#field_ident)).map_or_else(
                            || ::std::result::Result::Ok(::std::default::Default::default()),
                            |v| #crate_path::FromJs::from_js(v, runtime),
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
        impl #crate_path::FromJs for #struct_ident {
            fn from_js(
                value: ::quickjs_runtime::values::JsValueFacade,
                runtime: &::std::sync::Arc<::quickjs_runtime::facades::QuickJsRuntimeFacade>,
            ) -> ::std::result::Result<Self, #crate_path::JsError> {
                match value {
                    ::quickjs_runtime::values::JsValueFacade::JsObject {
                        ref cached_object
                    } => cached_object
                        .get_object_sync()
                        .map_err(::std::convert::Into::into)
                        .and_then(|mut object| {
                            ::std::result::Result::Ok(Self {
                                #(#fields),*
                            })
                        }),
                    _ => ::std::result::Result::Err(#crate_path::JsError::FromJs {
                        from: value.get_value_type().to_string(),
                        to: ::std::stringify!(#struct_ident),
                        message: ::std::option::Option::Some("expected object".to_string()),
                    }),
                }
            }
        }
    }
}

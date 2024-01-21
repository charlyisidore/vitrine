//! `FromJs` derive macro.

use proc_macro::TokenStream;
use quote::quote;

use super::VitrineAttribute;

pub fn impl_from_js_macro(ast: &syn::DeriveInput) -> TokenStream {
    let from_js_trait = quote!(crate::util::from_js::FromJs);

    let struct_ident = &ast.ident;

    let syn::Data::Struct(ref data) = ast.data else {
        return syn::Error::new(struct_ident.span(), "Only structs can derive `FromJs`")
            .to_compile_error()
            .into();
    };

    let syn::Fields::Named(ref fields) = data.fields else {
        return syn::Error::new(
            struct_ident.span(),
            "Only named structs can derive `FromJs`",
        )
        .to_compile_error()
        .into();
    };

    let fields =
        fields.named.iter().map(|field| {
            let field_ident = field.ident.as_ref().unwrap();
            let field_ident_str = field_ident.to_string();

            // Get supported attributes
            let field_attrs: Vec<VitrineAttribute> = field
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("vitrine"))
                .map(|attr| attr.parse_args())
                .collect::<syn::Result<_>>()
                .unwrap();

            field_attrs
            .iter()
            .find_map(|attr| match attr {
                VitrineAttribute::Default(function) => {
                let unwrap_fn = match function {
                        // `vitrine(default = "path")`
                        Some(function) => quote!(.unwrap_or_else(|| #function())),
                        // `vitrine(default)`
                        None => quote!(.unwrap_or_default()),
                    };

                    Some(quote!(
                        #field_ident: object
                            .remove(#field_ident_str)
                            .map(|v| #from_js_trait::from_js(
                                v,
                                ::std::sync::Arc::clone(&runtime),
                            ))
                            .transpose()
                            .map_err(
                                |error| error.context(format!("In field {}", #field_ident_str))
                            )?
                            #unwrap_fn
                    ))
                },
                VitrineAttribute::Skip => {
                    // `vitrine(skip)`
                    Some(quote!(
                        #field_ident: ::std::default::Default::default()
                    ))
                },
            })
            .unwrap_or_else(|| quote!(
                #field_ident: #from_js_trait::from_js(
                    object
                        .remove(#field_ident_str)
                        .unwrap_or_else(|| ::quickjs_runtime::values::JsValueFacade::Undefined),
                    ::std::sync::Arc::clone(&runtime),
                )
                    .map_err(|error| error.context(format!("In field {}", #field_ident_str)))?
            ))
        });

    return quote!(
        impl #from_js_trait for #struct_ident {
            fn from_js(
                value: ::quickjs_runtime::values::JsValueFacade,
                runtime: ::std::sync::Arc<::quickjs_runtime::facades::QuickJsRuntimeFacade>
            ) -> ::anyhow::Result<Self> {
                match value {
                    ::quickjs_runtime::values::JsValueFacade::JsObject {
                        cached_object
                    } => cached_object
                        .get_object_sync()
                        .map_err(|error| error.into())
                        .and_then(|mut object| {
                            Ok(Self {
                                #(#fields,)*
                            })
                        }),
                    _ => Err(::anyhow::anyhow!(
                        "Expected object, received {}",
                        value.stringify()
                    )),
                }
            }
        }
    )
    .into();
}

//! `FromRhai` derive macro.

use proc_macro::TokenStream;
use quote::quote;

use super::VitrineAttribute;

pub fn impl_from_rhai_macro(ast: &syn::DeriveInput) -> TokenStream {
    let from_rhai_trait = quote!(crate::util::from_rhai::FromRhai);

    let struct_ident = &ast.ident;

    let syn::Data::Struct(ref data) = ast.data else {
        return syn::Error::new(struct_ident.span(), "Only structs can derive `FromRhai`")
            .to_compile_error()
            .into();
    };

    let syn::Fields::Named(ref fields) = data.fields else {
        return syn::Error::new(
            struct_ident.span(),
            "Only named structs can derive `FromRhai`",
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
                .unwrap_or_default();

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
                            #field_ident: map
                                .get(#field_ident_str)
                                .map(|v| #from_rhai_trait::from_rhai(
                                    v,
                                    ::std::sync::Arc::clone(&engine),
                                    ::std::sync::Arc::clone(&ast),
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
                    #field_ident: #from_rhai_trait::from_rhai(
                        map
                            .get(#field_ident_str)
                            .unwrap_or_else(|| &::rhai::Dynamic::UNIT),
                        ::std::sync::Arc::clone(&engine),
                        ::std::sync::Arc::clone(&ast),
                    )
                        .map_err(|error| error.context(format!("In field {}", #field_ident_str)))?
                ))
        });

    return quote!(
        impl crate::util::from_rhai::FromRhai for #struct_ident {
            fn from_rhai(
                value: &::rhai::Dynamic,
                engine: ::std::sync::Arc<::rhai::Engine>,
                ast: ::std::sync::Arc<::rhai::AST>,
            ) -> ::anyhow::Result<Self> {
                let map = value.to_owned().try_cast::<::rhai::Map>().ok_or_else(|| {
                    ::anyhow::anyhow!(
                        "Expected Map, received {}",
                        value.type_name()
                    )
                })?;

                Ok(Self {
                    #(#fields,)*
                })
            }
        }
    )
    .into();
}

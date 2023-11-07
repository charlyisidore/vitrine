//! `FromRhai` derive macro.

use proc_macro::TokenStream;
use quote::quote;

pub fn impl_from_rhai_macro(ast: &syn::DeriveInput) -> TokenStream {
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

    let fields = fields.named.iter().map(|field| {
        let field_ident = field.ident.as_ref().unwrap();
        let field_ident_str = field_ident.to_string();

        quote!(
            #field_ident: crate::util::from_rhai::FromRhai::from_rhai(
                map.get(#field_ident_str).unwrap_or_else(|| &::rhai::Dynamic::UNIT),
                ::std::sync::Arc::clone(&engine),
                ::std::sync::Arc::clone(&ast)
            ).map_err(|error| error.context(
                format!("In field {}", #field_ident_str))
            )?
        )
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

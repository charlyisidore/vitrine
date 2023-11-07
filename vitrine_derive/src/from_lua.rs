//! `FromLua` derive macro.

use proc_macro::TokenStream;
use quote::quote;

pub fn impl_from_lua_macro(ast: &syn::DeriveInput) -> TokenStream {
    let struct_ident = &ast.ident;

    let syn::Data::Struct(ref data) = ast.data else {
        return syn::Error::new(struct_ident.span(), "Only structs can derive `FromLua`")
            .to_compile_error()
            .into();
    };

    let syn::Fields::Named(ref fields) = data.fields else {
        return syn::Error::new(
            struct_ident.span(),
            "Only named structs can derive `FromLua`",
        )
        .to_compile_error()
        .into();
    };

    let fields = fields.named.iter().map(|field| {
        let field_ident = field.ident.as_ref().unwrap();
        let field_ident_str = field_ident.to_string();

        quote!(
            #field_ident: crate::util::from_lua::FromLua::from_lua(
                table.get(#field_ident_str)
                    .map_err(|error| ::anyhow::anyhow!(error)
                        .context(format!("In field {}", #field_ident_str)))?,
                lua
            )
            .map_err(|error| error.context(format!("In field {}", #field_ident_str)))?
        )
    });

    return quote!(
        impl crate::util::from_lua::FromLua for #struct_ident {
            fn from_lua(value: ::mlua::Value, lua: &::mlua::Lua) -> ::anyhow::Result<Self> {
                let table = value.as_table().ok_or_else(|| {
                    ::anyhow::anyhow!(
                        "Expected table, received {}",
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

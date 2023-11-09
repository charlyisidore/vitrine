//! `FromLua` derive macro.

use proc_macro::TokenStream;
use quote::quote;

use super::VitrineAttribute;

pub fn impl_from_lua_macro(ast: &syn::DeriveInput) -> TokenStream {
    let from_lua_trait = quote!(crate::util::from_lua::FromLua);

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

        // Get supported attributes
        let field_attrs: Vec<VitrineAttribute> = field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("vitrine"))
            .map(|attr| attr.parse_args())
            .collect::<syn::Result<_>>()
            .unwrap();

        if field_attrs
            .iter()
            .find(|attr| match attr {
                VitrineAttribute::Skip => true,
                _ => false,
            })
            .is_some()
        {
            // `vitrine(skip)`
            return quote!(#field_ident: ::std::default::Default::default());
        }

        let and_then_fn = field_attrs
            .iter()
            .find_map(|attr| match attr {
                VitrineAttribute::Default(function) => match function {
                    // `vitrine(default = "path")`
                    Some(function) => Some(quote!(
                        |v| match v {
                            ::mlua::Value::Nil => Ok(#function()),
                            v => #from_lua_trait::from_lua(v, lua),
                        }
                    )),
                    // `vitrine(default)`
                    None => Some(quote!(|v| match v {
                        ::mlua::Value::Nil => Ok(::std::default::Default::default()),
                        v => #from_lua_trait::from_lua(v, lua),
                    })),
                },
                VitrineAttribute::Skip => unreachable!(),
            })
            .unwrap_or_else(|| quote!(|v| #from_lua_trait::from_lua(v, lua)));

        quote!(
            #field_ident: table
                .get(#field_ident_str)
                .map_err(|error| ::anyhow::anyhow!(error))
                .and_then(#and_then_fn)
                .map_err(|error| error.context(format!("In field {}", #field_ident_str)))?
        )
    });

    return quote!(
        impl #from_lua_trait for #struct_ident {
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

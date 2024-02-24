//! `FromLua` derive macro.

use proc_macro2::TokenStream;
use quote::quote;

use crate::VitrineAttribute;

/// Expand `FromLua` derive macro.
pub fn expand_from_lua(input: &syn::DeriveInput) -> TokenStream {
    let crate_path = quote! { ::vitrine::util::eval::lua };

    let struct_ident = &input.ident;

    let syn::Data::Struct(data) = &input.data else {
        return syn::Error::new(struct_ident.span(), "Only structs can derive `FromLua`")
            .to_compile_error();
    };

    let syn::Fields::Named(fields) = &data.fields else {
        return syn::Error::new(
            struct_ident.span(),
            "Only named structs can derive `FromLua`",
        )
        .to_compile_error();
    };

    let fields = fields.named.iter().map(|field| {
        let field_ident = field.ident.as_ref().unwrap();

        let err_with_field = quote! {
            |source| #crate_path::LuaError::WithField {
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
                        #field_ident: #crate_path::FromLua::from_lua(
                            table.get::<_, ::mlua::Value>(::std::stringify!(#field_ident))?,
                            lua,
                        ).map_err(#err_with_field)?
                    }
                },
                |attr| match attr {
                    VitrineAttribute::Default(path) => match path {
                        // `vitrine(default = "path")`
                        Some(path) => quote! {
                            #field_ident: match table.get::<_, ::mlua::Value>(
                                ::std::stringify!(#field_ident)
                            )? {
                                ::mlua::Nil => #path(),
                                v => #crate_path::FromLua::from_lua(v, lua)
                                    .map_err(#err_with_field)?,
                            }
                        },
                        // `vitrine(default)`
                        None => quote! {
                            #field_ident: match table.get::<_, ::mlua::Value>(
                                ::std::stringify!(#field_ident)
                            )? {
                                ::mlua::Nil => ::std::default::Default::default(),
                                v => #crate_path::FromLua::from_lua(v, lua)
                                    .map_err(#err_with_field)?,
                            }
                        },
                    },
                    VitrineAttribute::Skip => {
                        // vitrine(skip)
                        quote! {
                            #field_ident: ::std::default::Default::default()
                        }
                    },
                },
            )
    });

    quote! {
        impl #crate_path::FromLua for #struct_ident {
            fn from_lua(value: ::mlua::Value, lua: &::mlua::Lua)
                -> ::std::result::Result<Self, #crate_path::LuaError> {
                    match value {
                        ::mlua::Value::Table(table) => ::std::result::Result::Ok(Self {
                            #(#fields),*
                        }),
                        _ => ::std::result::Result::Err(::mlua::Error::FromLuaConversionError {
                            from: value.type_name(),
                            to: ::std::stringify!(#struct_ident),
                            message: ::std::option::Option::Some("expected table".to_string()),
                        }
                        .into()),
                    }
                }
        }
    }
}

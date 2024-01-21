//! Procedural macros for Vitrine.

mod from_js;
mod from_lua;
mod from_rhai;

use proc_macro::TokenStream;

/// Derive `FromJs` for a struct.
#[proc_macro_derive(FromJs, attributes(vitrine))]
pub fn from_js_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input);

    from_js::impl_from_js_macro(&ast)
}

/// Derive `FromLua` for a struct.
#[proc_macro_derive(FromLua, attributes(vitrine))]
pub fn from_lua_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input);

    from_lua::impl_from_lua_macro(&ast)
}

/// Derive `FromRhai` for a struct.
#[proc_macro_derive(FromRhai, attributes(vitrine))]
pub fn from_rhai_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input);

    from_rhai::impl_from_rhai_macro(&ast)
}

/// A parsed `#[vitrine(...)]` attribute.
#[derive(Debug)]
pub(self) enum VitrineAttribute {
    Default(Option<syn::Ident>),
    Skip,
}

impl syn::parse::Parse for VitrineAttribute {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;

        match name.to_string().as_str() {
            "default" => {
                if input.parse::<syn::Token![=]>().is_ok() {
                    // vitrine(default = "path")
                    let value: syn::LitStr = input.parse()?;
                    let ident: syn::Ident = value.parse()?;
                    Ok(Self::Default(Some(ident)))
                } else {
                    // vitrine(default)
                    Ok(Self::Default(None))
                }
            },
            "skip" => {
                // vitrine(skip)
                Ok(Self::Skip)
            },
            _ => Err(syn::Error::new_spanned(
                name.to_owned(),
                format!("Unknown vitrine attribute `{}`", name.to_string()),
            )),
        }
    }
}

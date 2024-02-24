//! Procedural macros for Vitrine.

mod from_js;
mod from_lua;
mod from_rhai;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Derive `FromJs` for a struct.
#[proc_macro_derive(FromJs, attributes(vitrine))]
pub fn from_js_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);

    from_js::expand_from_js(&input).into()
}

/// Derive `FromLua` for a struct.
#[proc_macro_derive(FromLua, attributes(vitrine))]
pub fn from_lua_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);

    from_lua::expand_from_lua(&input).into()
}

/// Derive `FromRhai` for a struct.
#[proc_macro_derive(FromRhai, attributes(vitrine))]
pub fn from_rhai_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);

    from_rhai::expand_from_rhai(&input).into()
}

/// Derive `VitrineNoop` for a struct.
///
/// This macro does not write any code. It allows to use `#[vitrine]` attributes
/// when none of the other derive macros is used.
#[proc_macro_derive(VitrineNoop, attributes(vitrine))]
pub fn vitrine_noop_derive(_: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// A parsed `#[vitrine(...)]` attribute.
#[derive(Debug)]
enum VitrineAttribute {
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
                format!("Unknown vitrine attribute `{}`", name),
            )),
        }
    }
}

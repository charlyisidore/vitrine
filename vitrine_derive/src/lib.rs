//! Procedural macros for Vitrine.

mod from_lua;
mod from_rhai;

use proc_macro::TokenStream;

/// Derive `FromLua` for a struct.
#[proc_macro_derive(FromLua)]
pub fn from_lua_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input);

    from_lua::impl_from_lua_macro(&ast)
}

/// Derive `FromRhai` for a struct.
#[proc_macro_derive(FromRhai)]
pub fn from_rhai_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input);

    from_rhai::impl_from_rhai_macro(&ast)
}

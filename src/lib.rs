//! A scriptable static site generator.
//!
//! Vitrine can be used as both a CLI and a library.
//!
//! # Feature flags
//!
//! - `default`: Enable `minijinja`, `mlua`, `rhai`, `tera`, and `v8` feature
//!   flags.
//! - `minijinja`: Enable minijinja (Jinja) layout engine.
//! - `mlua`: Enable mlua (Lua) script engine.
//! - `rhai`: Enable Rhai script engine.
//! - `tera`: Enable Tera layout engine.
//! - `v8`: Enable v8 (JavaScript) script engine.

#![warn(missing_docs)]

extern crate self as vitrine;

pub mod build;
pub mod config;
pub mod serve;
pub mod util;
pub mod watch;

pub use build::build;
pub use config::Config;
pub use serve::serve;
pub use util::url::{Url, UrlPath};
pub use watch::watch;

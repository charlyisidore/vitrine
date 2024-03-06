//! A scriptable static site generator.
//!
//! Vitrine can be used as both a CLI and a library.
//!
//! # Feature flags
//!
//! - `default`: Enable `jinja`, `js`, `lua`, `rhai`, and `tera` feature flags.
//! - `jinja`: Enable Jinja layout engine.
//! - `js`: Enable JavaScript script engine.
//! - `lua`: Enable Lua script engine.
//! - `rhai`: Enable Rhai script engine.
//! - `tera`: Enable Tera layout engine.

#![warn(missing_docs)]

extern crate self as vitrine;

pub mod config;
pub mod util;

pub use config::Config;

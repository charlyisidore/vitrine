//! Hackable static site generator.

#![warn(missing_docs)]

extern crate self as vitrine;

mod build;
pub mod cli;
pub mod config;
mod serve;
mod util;
mod watch;

pub use clap;

pub(crate) use crate::{
    build::{File, FileContent, Image, Script, Site, Style, TaxonomyItem},
    util::channel::ReceiverExt,
};
pub use crate::{
    build::{Page, build},
    config::{Config, FeedConfig, FeedPersonConfig, SitemapConfig},
    serve::serve,
    util::{
        date_time::DateTime,
        path::PathExt,
        uri::{UriReferenceString, UriRelativeString, UriString},
        value::{Value, to_value},
    },
    watch::watch,
};

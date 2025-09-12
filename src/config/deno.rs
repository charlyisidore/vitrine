//! Load configuration from TypeScript or JavaScript.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    thread::JoinHandle,
};

use anyhow::Result;
use async_channel::unbounded;
use serde::Deserialize;
use serde_json::Value;
use vitrine_deno::{
    CliFactory,
    args::{Flags, PermissionFlags},
    deno_runtime::{
        WorkerExecutionMode,
        deno_core::{
            JsRuntime,
            anyhow::anyhow,
            resolve_url_or_path,
            serde_v8::{from_v8, to_v8},
            v8,
        },
    },
};

use crate::{Config, FeedConfig, FeedPersonConfig, ReceiverExt, SitemapConfig, UriRelativeString};

/// Deserializable configuration.
#[derive(Default, Deserialize)]
pub struct UserConfig {
    /// See [`Config::base_url`].
    #[serde(default)]
    pub base_url: String,

    /// See [`Config::input_dir`].
    pub input_dir: Option<PathBuf>,

    /// See [`Config::output_dir`].
    pub output_dir: Option<PathBuf>,

    /// See [`Config::layout_dir`].
    pub layout_dir: Option<PathBuf>,

    /// See [`Config::ignore_paths`].
    #[serde(default)]
    pub ignore_paths: HashSet<PathBuf>,

    /// See [`Config::copy_paths`].
    #[serde(default)]
    pub copy_paths: HashMap<PathBuf, UriRelativeString>,

    /// See [`Config::site_data`].
    #[serde(default)]
    pub site_data: serde_json::Value,

    /// See [`Config::taxonomies`].
    #[serde(default)]
    pub taxonomies: Vec<String>,

    /// See [`Config::default_lang`].
    pub default_lang: Option<String>,

    /// See [`Config::markdown_plugins`].
    #[serde(default)]
    pub markdown_plugins: Vec<String>,

    /// See [`Config::feeds`].
    #[serde(default)]
    pub feeds: Vec<FeedUserConfig>,

    /// See [`Config::sitemap`].
    pub sitemap: Option<SitemapUserConfig>,
}

/// See [`FeedConfig`].
#[derive(Deserialize)]
pub struct FeedUserConfig {
    /// See [`FeedConfig::url`].
    pub url: UriRelativeString,

    /// See [`FeedConfig::author`].
    #[serde(default)]
    pub author: Vec<FeedPersonUserConfig>,

    /// See [`FeedConfig::category`].
    #[serde(default)]
    pub category: Vec<String>,

    /// See [`FeedConfig::contributor`].
    #[serde(default)]
    pub contributor: Vec<FeedPersonUserConfig>,

    /// See [`FeedConfig::generator`].
    pub generator: Option<String>,

    /// See [`FeedConfig::icon`].
    pub icon: Option<String>,

    /// See [`FeedConfig::id`].
    pub id: Option<String>,

    /// See [`FeedConfig::logo`].
    pub logo: Option<String>,

    /// See [`FeedConfig::rights`].
    pub rights: Option<String>,

    /// See [`FeedConfig::subtitle`].
    pub subtitle: Option<String>,

    /// See [`FeedConfig::title`].
    pub title: String,

    /// See [`FeedConfig::updated`].
    pub updated: Option<String>,
}

/// See [`FeedPersonConfig`].
#[derive(Deserialize)]
pub struct FeedPersonUserConfig {
    /// See [`FeedPersonConfig::name`].
    pub name: String,

    /// See [`FeedPersonConfig::uri`].
    pub uri: Option<String>,

    /// See [`FeedPersonConfig::email`].
    pub email: Option<String>,
}

/// See [`SitemapConfig`].
#[derive(Deserialize)]
#[serde(untagged)]
pub enum SitemapUserConfig {
    /// Boolean.
    Bool(bool),
    /// Object.
    Object {
        /// See [`SitemapConfig::changefreq`].
        changefreq: Option<String>,

        /// See [`SitemapConfig::priority`].
        priority: Option<f64>,

        /// See [`SitemapConfig::url_prefix`].
        #[serde(default)]
        url_prefix: String,

        /// See [`SitemapConfig::url`].
        url: Option<UriRelativeString>,
    },
}

/// Load a configuration from a Deno script.
pub fn from_path(
    path: impl AsRef<Path>,
    config: Config,
) -> Result<(Config, JoinHandle<Result<()>>)> {
    #[derive(Hash, PartialEq, Eq)]
    enum FunctionId {
        FeedFilter(usize),
        LayoutRender,
        LayoutFilter(String),
        LayoutFunction(String),
        LayoutTest(String),
        MarkdownRender,
    }

    enum FunctionArgs {
        FeedFilter(usize, Value),
        LayoutRender(String, Value),
        LayoutFilter(String, Value, Vec<Value>),
        LayoutFunction(String, Vec<Value>),
        LayoutTest(String, Value, Vec<Value>),
        MarkdownRender(String),
    }

    enum FunctionOutput {
        FeedFilter(bool),
        LayoutRender(String),
        LayoutFilter(Value),
        LayoutFunction(Value),
        LayoutTest(bool),
        MarkdownRender(String),
    }

    let path = path.as_ref().to_string_lossy().to_string();

    let (config_tx, config_rx) = unbounded::<Option<Config>>();
    let (args_tx, args_rx) = unbounded::<FunctionArgs>();
    let (output_tx, output_rx) = unbounded::<Result<FunctionOutput>>();

    let handle = std::thread::spawn(move || -> Result<()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        JsRuntime::init_platform(None, false);

        let main_module = resolve_url_or_path(&path, &std::env::current_dir()?)?;

        let cli_factory = CliFactory::from_flags(Arc::new(Flags {
            permissions: PermissionFlags {
                allow_all: true,
                ..Default::default()
            },
            ..Default::default()
        }));
        let worker_factory = rt.block_on(cli_factory.create_cli_main_worker_factory())?;

        let mut worker = rt
            .block_on(worker_factory.create_main_worker(
                WorkerExecutionMode::Run,
                main_module.clone(),
                Default::default(),
            ))?
            .into_main_worker();

        let module_id = rt.block_on(worker.preload_main_module(&main_module))?;
        rt.block_on(worker.evaluate_module(module_id))?;

        let namespace = worker.js_runtime.get_module_namespace(module_id)?;

        let scope = &mut worker.js_runtime.handle_scope();
        let namespace = v8::Local::new(scope, namespace);
        let default_key = v8::String::new(scope, "default").unwrap();
        let module = namespace
            .get(scope, default_key.into())
            .expect("no default module");

        let Some(object) = module.to_object(scope) else {
            return Err(anyhow!("error"));
        };

        let user_config: UserConfig = from_v8(scope, module)?;

        let mut config = Config {
            base_url: user_config.base_url,
            input_dir: user_config.input_dir.unwrap_or(config.input_dir),
            output_dir: user_config.output_dir.unwrap_or(config.output_dir),
            layout_dir: user_config.layout_dir,
            ignore_paths: user_config.ignore_paths,
            copy_paths: user_config.copy_paths,
            site_data: user_config.site_data,
            taxonomies: user_config.taxonomies,
            default_lang: user_config.default_lang,
            markdown_plugins: user_config.markdown_plugins,
            feeds: user_config
                .feeds
                .into_iter()
                .map(|feed| FeedConfig {
                    url: feed.url,
                    author: feed
                        .author
                        .into_iter()
                        .map(|person| FeedPersonConfig {
                            name: person.name,
                            uri: person.uri,
                            email: person.email,
                        })
                        .collect(),
                    category: feed.category,
                    contributor: feed
                        .contributor
                        .into_iter()
                        .map(|person| FeedPersonConfig {
                            name: person.name,
                            uri: person.uri,
                            email: person.email,
                        })
                        .collect(),
                    generator: feed.generator,
                    icon: feed.icon,
                    id: feed.id,
                    logo: feed.logo,
                    rights: feed.rights,
                    subtitle: feed.subtitle,
                    title: feed.title,
                    updated: feed.updated,
                    filter: Default::default(),
                })
                .collect(),
            sitemap: user_config.sitemap.and_then(|sitemap| match sitemap {
                SitemapUserConfig::Bool(sitemap) => sitemap.then(Default::default),
                SitemapUserConfig::Object {
                    changefreq,
                    priority,
                    url_prefix,
                    url,
                } => Some(SitemapConfig {
                    changefreq,
                    priority,
                    url_prefix,
                    url,
                }),
            }),
            markdown_render: Default::default(),
            layout_render: Default::default(),
            layout_filters: Default::default(),
            layout_functions: Default::default(),
            layout_tests: Default::default(),
            debug: Default::default(),
        };

        let mut functions = HashMap::new();

        let key = v8::String::new(scope, "feeds").unwrap();
        if let Some(array) = object
            .get(scope, key.into())
            .and_then(|v| v.try_cast::<v8::Array>().ok())
        {
            for (index, function) in (0..array.length()).filter_map(|index| {
                let key = v8::Number::new(scope, index.into());
                let value = array.get(scope, key.into())?;
                let object = value.to_object(scope)?;
                let key = v8::String::new(scope, "filter").unwrap();
                let function = object.get(scope, key.into())?;
                Some((
                    index.try_into().ok()?,
                    function.try_cast::<v8::Function>().ok()?,
                ))
            }) {
                functions.insert(FunctionId::FeedFilter(index), function);

                let args_tx = args_tx.clone();
                let output_rx = output_rx.clone();
                config.feeds.get_mut(index).unwrap().filter =
                    Some(Box::new(move |page: Value| -> Result<bool> {
                        args_tx.send_blocking(FunctionArgs::FeedFilter(index, page))?;
                        match output_rx.recv_blocking()?? {
                            FunctionOutput::FeedFilter(v) => Ok(v),
                            _ => unreachable!(),
                        }
                    }));
            }
        }

        let key = v8::String::new(scope, "layout_render").unwrap();
        if let Some(layout_render) = object
            .get(scope, key.into())
            .and_then(|v| v.try_cast::<v8::Function>().ok())
        {
            functions.insert(FunctionId::LayoutRender, layout_render);

            let args_tx = args_tx.clone();
            let output_rx = output_rx.clone();
            config.layout_render = Some(Box::new(
                move |layout: String, data: Value| -> Result<String> {
                    args_tx.send_blocking(FunctionArgs::LayoutRender(layout, data))?;
                    match output_rx.recv_blocking()?? {
                        FunctionOutput::LayoutRender(content) => Ok(content),
                        _ => unreachable!(),
                    }
                },
            ));
        }

        let key = v8::String::new(scope, "layout_filters").unwrap();
        if let Some(object) = object
            .get(scope, key.into())
            .and_then(|v| v.to_object(scope))
        {
            let keys = object
                .get_property_names(scope, Default::default())
                .unwrap();
            for (key, function) in (0..keys.length()).filter_map(|index| {
                let index = v8::Number::new(scope, index.into());
                let key = keys.get(scope, index.into())?;
                let function = object.get(scope, key)?;
                let key = key.to_string(scope)?;
                Some((
                    key.to_rust_string_lossy(scope),
                    function.try_cast::<v8::Function>().ok()?,
                ))
            }) {
                functions.insert(FunctionId::LayoutFilter(key.clone()), function);

                let args_tx = args_tx.clone();
                let output_rx = output_rx.clone();
                config.layout_filters.insert(
                    key.clone(),
                    Arc::new(move |value: Value, args: Vec<Value>| -> Result<Value> {
                        args_tx.send_blocking(FunctionArgs::LayoutFilter(
                            key.clone(),
                            value,
                            args,
                        ))?;
                        match output_rx.recv_blocking()?? {
                            FunctionOutput::LayoutFilter(v) => Ok(v),
                            _ => unreachable!(),
                        }
                    }),
                );
            }
        }

        let key = v8::String::new(scope, "layout_functions").unwrap();
        if let Some(object) = object
            .get(scope, key.into())
            .and_then(|v| v.to_object(scope))
        {
            let keys = object
                .get_property_names(scope, Default::default())
                .unwrap();
            for (key, layout_function) in (0..keys.length()).filter_map(|index| {
                let index = v8::Number::new(scope, index.into());
                let key = keys.get(scope, index.into())?;
                let function = object.get(scope, key)?;
                let key = key.to_string(scope)?;
                Some((
                    key.to_rust_string_lossy(scope),
                    function.try_cast::<v8::Function>().ok()?,
                ))
            }) {
                functions.insert(FunctionId::LayoutFunction(key.clone()), layout_function);

                let args_tx = args_tx.clone();
                let output_rx = output_rx.clone();
                config.layout_functions.insert(
                    key.clone(),
                    Arc::new(move |args: Vec<Value>| -> Result<Value> {
                        args_tx.send_blocking(FunctionArgs::LayoutFunction(key.clone(), args))?;
                        match output_rx.recv_blocking()?? {
                            FunctionOutput::LayoutFunction(v) => Ok(v),
                            _ => unreachable!(),
                        }
                    }),
                );
            }
        }

        let key = v8::String::new(scope, "layout_tests").unwrap();
        if let Some(object) = object
            .get(scope, key.into())
            .and_then(|v| v.to_object(scope))
        {
            let keys = object
                .get_property_names(scope, Default::default())
                .unwrap();
            for (key, function) in (0..keys.length()).filter_map(|index| {
                let index = v8::Number::new(scope, index.into());
                let key = keys.get(scope, index.into())?;
                let function = object.get(scope, key)?;
                let key = key.to_string(scope)?;
                Some((
                    key.to_rust_string_lossy(scope),
                    function.try_cast::<v8::Function>().ok()?,
                ))
            }) {
                functions.insert(FunctionId::LayoutTest(key.clone()), function);

                let args_tx = args_tx.clone();
                let output_rx = output_rx.clone();
                config.layout_tests.insert(
                    key.clone(),
                    Arc::new(move |value: Value, args: Vec<Value>| -> Result<bool> {
                        args_tx.send_blocking(FunctionArgs::LayoutTest(
                            key.clone(),
                            value,
                            args,
                        ))?;
                        match output_rx.recv_blocking()?? {
                            FunctionOutput::LayoutTest(v) => Ok(v),
                            _ => unreachable!(),
                        }
                    }),
                );
            }
        }

        let key = v8::String::new(scope, "markdown_render").unwrap();
        if let Some(function) = object
            .get(scope, key.into())
            .and_then(|v| v.try_cast::<v8::Function>().ok())
        {
            functions.insert(FunctionId::MarkdownRender, function);

            let args_tx = args_tx.clone();
            let output_rx = output_rx.clone();
            config.markdown_render = Some(Box::new(move |content: String| -> Result<String> {
                args_tx.send_blocking(FunctionArgs::MarkdownRender(content))?;
                match output_rx.recv_blocking()?? {
                    FunctionOutput::MarkdownRender(content) => Ok(content),
                    _ => unreachable!(),
                }
            }));
        }

        drop(args_tx);

        config_tx.send_blocking(Some(config))?;

        for args in args_rx.into_iter() {
            let scope = &mut v8::TryCatch::new(scope);
            let (id, args): (FunctionId, Vec<_>) = match args {
                FunctionArgs::FeedFilter(index, page) => {
                    (FunctionId::FeedFilter(index), [to_v8(scope, page)?].into())
                },
                FunctionArgs::LayoutRender(layout, data) => (
                    FunctionId::LayoutRender,
                    [to_v8(scope, layout)?, to_v8(scope, data)?].into(),
                ),
                FunctionArgs::LayoutFilter(key, value, args) => (
                    FunctionId::LayoutFilter(key),
                    [value]
                        .into_iter()
                        .chain(args)
                        .map(|v| to_v8(scope, v).map_err(Into::into))
                        .collect::<Result<_>>()?,
                ),
                FunctionArgs::LayoutFunction(key, args) => (
                    FunctionId::LayoutFunction(key),
                    args.into_iter()
                        .map(|v| to_v8(scope, v).map_err(Into::into))
                        .collect::<Result<_>>()?,
                ),
                FunctionArgs::LayoutTest(key, value, args) => (
                    FunctionId::LayoutTest(key),
                    [value]
                        .into_iter()
                        .chain(args)
                        .map(|v| to_v8(scope, v).map_err(Into::into))
                        .collect::<Result<_>>()?,
                ),
                FunctionArgs::MarkdownRender(content) => {
                    (FunctionId::MarkdownRender, [to_v8(scope, content)?].into())
                },
            };

            let function = functions.get(&id).unwrap();
            let function = v8::Global::new(scope, function);
            let args = args
                .into_iter()
                .map(|value| v8::Global::new(scope, value))
                .collect::<Vec<_>>();

            let result = rt.block_on(JsRuntime::scoped_call_with_args(
                scope,
                &function,
                args.as_slice(),
            ));

            let result = result
                .map(|value| v8::Local::new(scope, value))
                .map_err(Into::into)
                .and_then(|output| match id {
                    FunctionId::FeedFilter(..) => {
                        Ok(FunctionOutput::FeedFilter(from_v8(scope, output)?))
                    },
                    FunctionId::LayoutRender => {
                        Ok(FunctionOutput::LayoutRender(from_v8(scope, output)?))
                    },
                    FunctionId::LayoutFilter(..) => {
                        Ok(FunctionOutput::LayoutFilter(from_v8(scope, output)?))
                    },
                    FunctionId::LayoutFunction(..) => {
                        Ok(FunctionOutput::LayoutFunction(from_v8(scope, output)?))
                    },
                    FunctionId::LayoutTest(..) => {
                        Ok(FunctionOutput::LayoutTest(from_v8(scope, output)?))
                    },
                    FunctionId::MarkdownRender => {
                        Ok(FunctionOutput::MarkdownRender(from_v8(scope, output)?))
                    },
                });

            output_tx.send_blocking(result)?;
        }

        Ok(())
    });

    if let Some(config) = config_rx.recv_blocking().ok().flatten() {
        Ok((config, handle))
    } else {
        Err(handle.join().unwrap().unwrap_err())
    }
}

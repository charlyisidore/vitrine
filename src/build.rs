//! Build the site.

pub mod assets;
pub mod bundle_css;
pub mod bundle_html;
pub mod bundle_js;
pub mod input;
pub mod layout;
pub mod markdown;
pub mod metadata;
pub mod minify_css;
pub mod minify_html;
pub mod minify_js;
pub mod output;
pub mod scss;
pub mod syntax_highlight;
pub mod typescript;

use std::path::PathBuf;

use thiserror::Error;

use self::{
    assets::task::AssetsTask, bundle_css::task::BundleCssTask, bundle_html::task::BundleHtmlTask,
    bundle_js::task::BundleJsTask, input::task::InputTask, layout::task::LayoutTask,
    markdown::task::MarkdownTask, metadata::task::MetadataTask, minify_css::task::MinifyCssTask,
    minify_html::task::MinifyHtmlTask, minify_js::task::MinifyJsTask, output::task::OutputTask,
    scss::task::ScssTask,
};
use crate::{
    build::syntax_highlight::task::SyntaxHighlightTask,
    config::Config,
    util::{pipeline::Pipeline, url::UrlPath, value::Value},
};

/// List of build errors.
#[derive(Debug, Error)]
pub enum BuildError {
    /// Error while extracting HTML assets.
    #[error("failed to extract HTML assets")]
    Assets(#[source] self::assets::AssetsError),
    /// Error while bundling CSS.
    #[error("failed to bundle CSS")]
    BundleCss(#[source] self::bundle_css::BundleCssError),
    /// Error while bundling HTML.
    #[error("failed to bundle HTML")]
    BundleHtml(#[source] self::bundle_html::BundleHtmlError),
    /// Error while bundling JavaScript.
    #[error("failed to bundle JavaScript")]
    BundleJs(#[source] self::bundle_js::BundleJsError),
    /// Error while walking input files
    #[error("failed to walk input files")]
    Input(#[source] self::input::InputError),
    /// Error while rendering a layout.
    #[error("failed to render layout")]
    Layout(#[source] self::layout::LayoutError),
    /// Error while parsing Markdown.
    #[error("failed to parse Markdown")]
    Markdown(#[source] self::markdown::MarkdownError),
    /// Error while parsing metadata.
    #[error("failed to parse metadata")]
    Metadata(#[source] self::metadata::MetadataError),
    /// Error while minifying CSS.
    #[error("failed to minify CSS")]
    MinifyCss(#[source] self::minify_css::MinifyCssError),
    /// Error while minifying HTML.
    #[error("failed to minify HTML")]
    MinifyHtml(#[source] self::minify_html::MinifyHtmlError),
    /// Error while minifying JavaScript.
    #[error("failed to minify JavaScript")]
    MinifyJs(#[source] self::minify_js::MinifyJsError),
    /// Error while creating the layout engine.
    #[error("failed to initialize the layout engine")]
    NewLayout(#[source] self::layout::LayoutError),
    /// Error while creating the Markdown parser.
    #[error("failed to initialize the markdown parser")]
    NewMarkdown(#[source] self::markdown::MarkdownError),
    /// Error while writing output files
    #[error("failed to write output files")]
    Output(#[source] self::output::OutputError),
    /// Error while compiling SCSS.
    #[error("failed to compile SCSS")]
    Scss(#[source] self::scss::ScssError),
    /// Error while creating syntax highlight themes.
    #[error("failed to generate syntax highlight themes")]
    SyntaxHighlight(#[source] self::syntax_highlight::SyntaxHighlightError),
}

/// A page entry.
///
/// A page represents a future HTML file.
#[derive(Debug, Default)]
pub struct Page {
    /// Input file path from which the entry comes from.
    input_path: PathBuf,

    /// URL from which the entry will be accessible.
    ///
    /// The URL determines the output file name (e.g. `/blog/` outputs
    /// `/blog/index.html`).
    url: UrlPath,

    /// Content of the entry.
    content: String,

    /// Page data.
    data: Value,
}

/// An image entry.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Image {
    /// Input file path from which the entry comes from.
    input_path: PathBuf,

    /// URL from which the entry will be accessible.
    url: UrlPath,

    /// Image target width.
    width: Option<u32>,

    /// Image target height.
    height: Option<u32>,
}

/// A script entry.
///
/// A script represents a future JavaScript file.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Script {
    /// Input file path from which the entry comes from.
    input_path: PathBuf,

    /// URL from which the entry will be accessible.
    url: UrlPath,

    /// Content of the entry.
    content: String,
}

/// A style entry.
///
/// A style represents a future CSS file.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Style {
    /// Input file path from which the entry comes from.
    input_path: PathBuf,

    /// URL from which the entry will be accessible.
    url: UrlPath,

    /// Content of the entry.
    content: String,
}

/// An asset (image, script, or style).
#[derive(Debug)]
pub enum Asset {
    /// Image entry.
    Image(Image),
    /// Script entry.
    Script(Script),
    /// Style entry.
    Style(Style),
}

/// An entry (page or asset).
#[derive(Debug)]
pub enum Entry {
    /// Page entry.
    Page(Page),
    /// Image entry.
    Image(Image),
    /// Script entry.
    Script(Script),
    /// Style entry.
    Style(Style),
}

/// Build the site with given configuration.
pub fn build(config: &Config) -> Result<(), BuildError> {
    // Check if configuration is normalized
    debug_assert!(config
        .config_path
        .as_ref()
        .map_or(true, |path| path.is_absolute()));
    debug_assert!(config.input_dir.is_absolute());
    debug_assert!(config
        .output_dir
        .as_ref()
        .map_or(true, |path| path.is_absolute()));
    debug_assert!(config
        .layout_dir
        .as_ref()
        .map_or(true, |path| path.is_absolute()));

    let start_time = std::time::Instant::now();

    let walk_task = InputTask::new(config);
    let metadata_task = MetadataTask::new();
    let markdown_task = MarkdownTask::new(config).map_err(BuildError::NewMarkdown)?;
    let layout_task = LayoutTask::new(config).map_err(BuildError::NewLayout)?;
    let assets_task = AssetsTask::new(config);
    let minify_html_task = MinifyHtmlTask::new(config);

    let bundle_js_task = BundleJsTask::new();
    let minify_js_task = MinifyJsTask::new(config);

    let scss_task = ScssTask::new();
    let bundle_css_task = BundleCssTask::new();
    let minify_css_task = MinifyCssTask::new(config);

    let syntax_highlight_task = SyntaxHighlightTask::new(config);

    let bundle_html_task = BundleHtmlTask::new(config);
    let output_task = OutputTask::new(config);

    let (page_pipeline, image_pipeline, script_pipeline, style_pipeline) = Pipeline::new(walk_task)
        .map_err(BuildError::Input)?
        .pipe(metadata_task)
        .map_err(BuildError::Metadata)?
        .pipe(markdown_task)
        .map_err(BuildError::Markdown)?
        .pipe(layout_task)
        .map_err(BuildError::Layout)?
        .fork(assets_task)
        .map_err(BuildError::Assets)?;

    let script_pipeline = script_pipeline
        .pipe(bundle_js_task)
        .map_err(BuildError::BundleJs)?
        .pipe(minify_js_task)
        .map_err(BuildError::MinifyJs)?;

    let style_pipeline = style_pipeline
        .pipe(scss_task)
        .map_err(BuildError::Scss)?
        .pipe(syntax_highlight_task)
        .map_err(BuildError::SyntaxHighlight)?
        .pipe(bundle_css_task)
        .map_err(BuildError::BundleCss)?
        .pipe(minify_css_task)
        .map_err(BuildError::MinifyCss)?;

    let (page_pipeline, asset_pipeline) = Pipeline::<()>::multiplex(
        (
            page_pipeline,
            image_pipeline,
            script_pipeline,
            style_pipeline,
        ),
        bundle_html_task,
    )
    .map_err(BuildError::BundleHtml)?;

    let page_pipeline = page_pipeline
        .pipe(minify_html_task)
        .map_err(BuildError::MinifyHtml)?;

    let num_output_files = Pipeline::merge((page_pipeline, asset_pipeline), output_task)
        .map_err(BuildError::Output)?
        .into_iter()
        .count();

    let duration = start_time.elapsed().as_secs_f64();

    println!(
        "Wrote {} files in {:.2} seconds",
        num_output_files, duration
    );

    Ok(())
}

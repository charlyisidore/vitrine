//! Application errors.

use std::path::PathBuf;

/// Enumerates application errors.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("While loading configuration from {config_path:?}")]
    LoadConfig {
        config_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("While initializing the layout engine")]
    NewLayoutEngine { source: anyhow::Error },
    #[error("While reading global data file {input_path:?}")]
    ReadGlobalDataInput {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("While reading file {input_path:?}")]
    ReadInput {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("In {input_path:?} while parsing the front matter")]
    ParseFrontMatter {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("In {input_path:?} while parsing data")]
    ParseCascadeData {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("In {input_path:?} while normalizing URL")]
    NormalizeUrl {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("In {input_path:?} while compiling SCSS")]
    CompileScss {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("While creating syntax highlight CSS stylesheet")]
    CreateSyntaxHighlightStylesheet { source: anyhow::Error },
    #[error("While grouping entries using taxonomies")]
    GroupTaxonomies { source: anyhow::Error },
    #[error("While bundling contents")]
    BundleContents { source: anyhow::Error },
    #[error("In {input_path:?} while rendering layout {layout:?}")]
    RenderLayout {
        input_path: Option<PathBuf>,
        layout: Option<String>,
        source: anyhow::Error,
    },
    #[error("In {input_path:?} while rewriting URL")]
    RewriteUrl {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("In {input_path:?} while minifying CSS")]
    MinifyCss {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("In {input_path:?} while minifying HTML")]
    MinifyHtml {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("In {input_path:?} while minifying JavaScript")]
    MinifyJs {
        input_path: Option<PathBuf>,
        source: anyhow::Error,
    },
    #[error("While writing the file {output_path:?}")]
    WriteOutput {
        output_path: PathBuf,
        source: anyhow::Error,
    },
}

//! Read JavaScript script files.

use std::{path::Path, sync::Arc};

use quickjs_runtime::{builder::QuickJsRuntimeBuilder, jsutils::Script};

use crate::util::from_js::FromJs;

/// Read data from a JavaScript script.
pub(crate) fn read_file<T, P>(path: P) -> anyhow::Result<T>
where
    T: FromJs,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    read_str(
        content,
        path.to_str()
            .ok_or_else(|| anyhow::anyhow!("Path is not unicode"))?,
    )
}

/// Read data from a JavaScript script.
pub(crate) fn read_str<T, S, P>(content: S, path: P) -> anyhow::Result<T>
where
    T: FromJs,
    S: AsRef<str>,
    P: AsRef<str>,
{
    let content = content.as_ref();
    let path = path.as_ref();

    let runtime = Arc::new(QuickJsRuntimeBuilder::new().build());

    let result = runtime.eval_sync(None, Script::new(path, content))?;

    let result = FromJs::from_js(result, runtime)?;

    Ok(result)
}

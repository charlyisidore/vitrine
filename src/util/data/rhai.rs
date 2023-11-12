//! Read Rhai script files.

use std::{path::Path, sync::Arc};

use crate::util::from_rhai::FromRhai;

/// Read data from a Rhai script.
pub(crate) fn read_file<T, P>(path: P) -> anyhow::Result<T>
where
    T: FromRhai,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    read_str(content)
}

/// Read data from a Rhai script.
pub(crate) fn read_str<T, S>(content: S) -> anyhow::Result<T>
where
    T: FromRhai,
    S: AsRef<str>,
{
    let content = content.as_ref();

    // Initialize the rhai engine
    let engine = Arc::new(rhai::Engine::new());

    // Compile the script
    let ast = Arc::new(engine.compile(content)?);

    // Execute the script
    let result: rhai::Dynamic = engine.eval_ast(&ast)?;

    let result = T::from_rhai(&result, engine, ast)?;

    Ok(result)
}

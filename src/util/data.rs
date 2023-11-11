//! Utility functions for serializable data.

pub(crate) mod json;
pub(crate) mod toml;
pub(crate) mod yaml;

use serde::Serialize;

/// Perform a shallow merge of two objects.
pub(crate) fn shallow_merge<T1, T2>(v1: T1, v2: T2) -> anyhow::Result<serde_json::Value>
where
    T1: Serialize,
    T2: Serialize,
{
    let mut v1 = serde_json::to_value(v1)?
        .as_object_mut()
        .cloned()
        .unwrap_or_default();

    let v2 = serde_json::to_value(v2)?
        .as_object()
        .cloned()
        .unwrap_or_default();

    v1.extend(v2);

    Ok(v1.into())
}

//! Read Lua script files.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use crate::util::from_lua::FromLua;

/// Read data from a Lua script.
pub(crate) fn read_file<T, P>(path: P) -> anyhow::Result<T>
where
    T: FromLua,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    read_str(content)
}

/// Read data from a Lua script.
pub(crate) fn read_str<T, S>(content: S) -> anyhow::Result<T>
where
    T: FromLua,
    S: AsRef<str>,
{
    let content = content.as_ref();

    // Call `unsafe_new()` to allow loading C modules
    let lua = unsafe { mlua::Lua::unsafe_new() };

    // `Lua` is not `Sync`, so we wrap it in `Arc<Mutex>`
    let lua_mutex = Arc::new(Mutex::new(lua));
    let lua = lua_mutex.lock().unwrap();

    // Save the mutex in Lua's context, we can retrieve it with `lua.app_data_ref()`
    lua.set_app_data(Arc::clone(&lua_mutex));

    // Execute the script
    let result: mlua::Value = lua.load(content).eval()?;

    let result = T::from_lua(result, &lua)?;

    Ok(result)
}

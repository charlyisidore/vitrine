use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use std::path::Path;
/// Checks if the path has an extension Deno supports for script execution.
pub fn is_script_ext(path: &Path) -> bool {
  if let Some(ext) = get_extension(path) {
    matches!(
      ext.as_str(),
      "ts" | "tsx" | "js" | "jsx" | "mjs" | "mts" | "cjs" | "cts"
    )
  } else {
    false
  }
}
/// Get the extension of a file in lowercase.
pub fn get_extension(file_path: &Path) -> Option<String> {
  file_path
    .extension()
    .and_then(|e| e.to_str())
    .map(|e| e.to_lowercase())
}
/// TypeScript figures out the type of file based on the extension, but we take
/// other factors into account like the file headers. The hack here is to map the
/// specifier passed to TypeScript to a new specifier with the file extension.
pub fn mapped_specifier_for_tsc(
  specifier: &ModuleSpecifier,
  media_type: MediaType,
) -> Option<String> {
  let ext_media_type = MediaType::from_specifier(specifier);
  if media_type != ext_media_type {
    if media_type != MediaType::Dts
      && media_type == MediaType::TypeScript
      && specifier
        .path()
        .split('/')
        .next_back()
        .map(|last| last.contains(".d."))
        .unwrap_or(false)
    {
      let mut path_parts = specifier
        .path()
        .split('/')
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
      let last_part = path_parts.last_mut().unwrap();
      *last_part = last_part.replace(".d.", "$d$");
      let mut specifier = specifier.clone();
      specifier.set_path(&path_parts.join("/"));
      Some(format!("{}{}", specifier, media_type.as_ts_extension()))
    } else {
      Some(format!("{}{}", specifier, media_type.as_ts_extension()))
    }
  } else {
    None
  }
}
/// For decoding percent-encodeing string
/// could be used for module specifier string literal of local modules,
/// or local file path to display `non-ASCII` characters correctly
/// # Examples
/// ```
/// use crate::util::path::to_percent_decoded_str;
///
/// let str = to_percent_decoded_str("file:///Users/path/to/%F0%9F%A6%95.ts");
/// assert_eq!(str, "file:///Users/path/to/🦕.ts");
/// ```
pub fn to_percent_decoded_str(s: &str) -> String {
  match percent_encoding::percent_decode_str(s).decode_utf8() {
    Ok(s) => s.to_string(),
    Err(_) => s.to_string(),
  }
}

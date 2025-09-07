use deno_core::error::AnyError;
use deno_lib::standalone::binary::SerializedWorkspaceResolverImportMap;
use eszip::EszipV2;
use jsonc_parser::ParseOptions;
pub mod hmr;
#[allow(unused)]
async fn load_import_map(
  eszips: &[EszipV2],
  specifier: &str,
) -> Result<SerializedWorkspaceResolverImportMap, AnyError> {
  let maybe_module = eszips
    .iter()
    .rev()
    .find_map(|eszip| eszip.get_import_map(specifier));
  let Some(module) = maybe_module else {
    return Err(AnyError::msg(format!("import map not found '{specifier}'")));
  };
  let base_url = deno_core::url::Url::parse(specifier).map_err(|err| {
    AnyError::msg(format!(
      "import map specifier '{specifier}' is not a valid url: {err}"
    ))
  })?;
  let bytes = module
    .source()
    .await
    .ok_or_else(|| AnyError::msg("import map not found '{specifier}'"))?;
  let text = String::from_utf8_lossy(&bytes);
  let json_value =
    jsonc_parser::parse_to_serde_value(&text, &ParseOptions::default())
      .map_err(|err| {
        AnyError::msg(format!("import map failed to parse: {err}"))
      })?
      .ok_or_else(|| AnyError::msg("import map is not valid JSON"))?;
  let import_map = import_map::parse_from_value(base_url, json_value)?;
  Ok(SerializedWorkspaceResolverImportMap {
    specifier: specifier.to_string(),
    json: import_map.import_map.to_json(),
  })
}

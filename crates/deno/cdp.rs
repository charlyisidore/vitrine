/// <https://chromedevtools.github.io/devtools-protocol/tot/>
use deno_core::serde_json::Value;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#method-awaitPromise>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AwaitPromiseArgs {
  pub promise_object_id: RemoteObjectId,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub return_by_value: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub generate_preview: Option<bool>,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#method-compileScript>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CompileScriptArgs {
  pub expression: String,
  #[serde(rename = "sourceURL")]
  pub source_url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub execution_context_id: Option<ExecutionContextId>,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#method-queryObjects>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct QueryObjectsArgs {
  pub prototype_object_id: RemoteObjectId,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub object_group: Option<String>,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#method-releaseObject>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ReleaseObjectArgs {
  pub object_id: RemoteObjectId,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#method-releaseObjectGroup>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ReleaseObjectGroupArgs {
  pub object_group: String,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#method-runScript>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct RunScriptArgs {
  pub script_id: ScriptId,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub execution_context_id: Option<ExecutionContextId>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub object_group: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub silent: Option<bool>,
  #[serde(
    rename = "includeCommandLineAPI",
    skip_serializing_if = "Option::is_none"
  )]
  pub include_command_line_api: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub return_by_value: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub generate_preview: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub await_promise: Option<bool>,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#method-setAsyncCallStackDepth>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct SetAsyncCallStackDepthArgs {
  pub max_depth: u64,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#type-RemoteObject>
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct RemoteObject {
  #[serde(rename = "type")]
  pub kind: String,
  #[serde(default, deserialize_with = "deserialize_some")]
  pub value: Option<Value>,
  pub unserializable_value: Option<UnserializableValue>,
  pub description: Option<String>,
  pub object_id: Option<RemoteObjectId>,
}
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
  T: Deserialize<'de>,
  D: Deserializer<'de>,
{
  Deserialize::deserialize(deserializer).map(Some)
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#type-ExceptionDetails>
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
  pub text: String,
  pub exception: Option<RemoteObject>,
}
impl ExceptionDetails {
  pub fn get_message_and_description(&self) -> (String, String) {
    let description = self
      .exception
      .clone()
      .and_then(|ex| ex.description)
      .unwrap_or_else(|| "undefined".to_string());
    (self.text.to_string(), description)
  }
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#type-RemoteObjectId>
pub type RemoteObjectId = String;
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#type-ExecutionContextId>
pub type ExecutionContextId = u64;
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#type-ScriptId>
pub type ScriptId = String;
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#type-UnserializableValue>
pub type UnserializableValue = String;
/// <https://chromedevtools.github.io/devtools-protocol/tot/Debugger/#method-setScriptSource>
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetScriptSourceResponse {
  pub status: Status,
  pub exception_details: Option<ExceptionDetails>,
}
#[derive(Debug, Deserialize)]
pub enum Status {
  Ok,
  CompileError,
  BlockedByActiveGenerator,
  BlockedByActiveFunction,
  BlockedByTopLevelEsModuleChange,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Debugger/#event-scriptParsed>
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptParsed {
  pub script_id: String,
  pub url: String,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Profiler/#type-CoverageRange>
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoverageRange {
  /// Start character index.
  #[serde(rename = "startOffset")]
  pub start_char_offset: usize,
  /// End character index.
  #[serde(rename = "endOffset")]
  pub end_char_offset: usize,
  pub count: i64,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Profiler/#type-FunctionCoverage>
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCoverage {
  pub function_name: String,
  pub ranges: Vec<CoverageRange>,
  pub is_block_coverage: bool,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Profiler/#type-ScriptCoverage>
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScriptCoverage {
  pub script_id: String,
  pub url: String,
  pub functions: Vec<FunctionCoverage>,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Profiler/#method-startPreciseCoverage>
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPreciseCoverageArgs {
  pub call_count: bool,
  pub detailed: bool,
  pub allow_triggered_updates: bool,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Profiler/#method-startPreciseCoverage>
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPreciseCoverageResponse {
  pub timestamp: f64,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Profiler/#method-takePreciseCoverage>
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakePreciseCoverageResponse {
  pub result: Vec<ScriptCoverage>,
  pub timestamp: f64,
}
#[derive(Debug, Deserialize)]
pub struct Notification {
  pub method: String,
  pub params: Value,
}
/// <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#event-exceptionThrown>
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionThrown {
  pub exception_details: ExceptionDetails,
}

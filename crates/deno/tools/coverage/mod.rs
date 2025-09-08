use crate::cdp;
use crate::tools::fmt::format_json;
use deno_core::InspectorPostMessageError;
use deno_core::InspectorPostMessageErrorKind;
use deno_core::LocalInspectorSession;
use deno_core::error::CoreError;
use deno_core::serde_json;
use deno_core::url::Url;
use deno_error::JsErrorBox;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;
pub struct CoverageCollector {
  pub dir: PathBuf,
  session: LocalInspectorSession,
}
impl CoverageCollector {
  pub fn new(dir: PathBuf, session: LocalInspectorSession) -> Self {
    Self { dir, session }
  }
  pub async fn start_collecting(
    &mut self,
  ) -> Result<(), InspectorPostMessageError> {
    self.enable_debugger().await?;
    self.enable_profiler().await?;
    self
      .start_precise_coverage(cdp::StartPreciseCoverageArgs {
        call_count: true,
        detailed: true,
        allow_triggered_updates: false,
      })
      .await?;
    Ok(())
  }
  pub async fn stop_collecting(&mut self) -> Result<(), CoreError> {
    fs::create_dir_all(&self.dir)?;
    let script_coverages = self.take_precise_coverage().await?.result;
    for script_coverage in script_coverages {
      if script_coverage.url.is_empty()
        || script_coverage.url.starts_with("ext:")
        || script_coverage.url.starts_with("[ext:")
        || script_coverage.url.starts_with("http:")
        || script_coverage.url.starts_with("https:")
        || script_coverage.url.starts_with("node:")
        || Url::parse(&script_coverage.url).is_err()
      {
        continue;
      }
      let filename = format!("{}.json", Uuid::new_v4());
      let filepath = self.dir.join(filename);
      let mut out = BufWriter::new(File::create(&filepath)?);
      let coverage = serde_json::to_string(&script_coverage)
        .map_err(JsErrorBox::from_err)?;
      let formatted_coverage =
        format_json(&filepath, &coverage, &Default::default())
          .ok()
          .flatten()
          .unwrap_or(coverage);
      out.write_all(formatted_coverage.as_bytes())?;
      out.flush()?;
    }
    self.disable_debugger().await?;
    self.disable_profiler().await?;
    Ok(())
  }
  async fn enable_debugger(&mut self) -> Result<(), InspectorPostMessageError> {
    self
      .session
      .post_message::<()>("Debugger.enable", None)
      .await?;
    Ok(())
  }
  async fn enable_profiler(&mut self) -> Result<(), InspectorPostMessageError> {
    self
      .session
      .post_message::<()>("Profiler.enable", None)
      .await?;
    Ok(())
  }
  async fn disable_debugger(
    &mut self,
  ) -> Result<(), InspectorPostMessageError> {
    self
      .session
      .post_message::<()>("Debugger.disable", None)
      .await?;
    Ok(())
  }
  async fn disable_profiler(
    &mut self,
  ) -> Result<(), InspectorPostMessageError> {
    self
      .session
      .post_message::<()>("Profiler.disable", None)
      .await?;
    Ok(())
  }
  async fn start_precise_coverage(
    &mut self,
    parameters: cdp::StartPreciseCoverageArgs,
  ) -> Result<cdp::StartPreciseCoverageResponse, InspectorPostMessageError> {
    let return_value = self
      .session
      .post_message("Profiler.startPreciseCoverage", Some(parameters))
      .await?;
    let return_object = serde_json::from_value(return_value).map_err(|e| {
      InspectorPostMessageErrorKind::JsBox(JsErrorBox::from_err(e)).into_box()
    })?;
    Ok(return_object)
  }
  async fn take_precise_coverage(
    &mut self,
  ) -> Result<cdp::TakePreciseCoverageResponse, InspectorPostMessageError> {
    let return_value = self
      .session
      .post_message::<()>("Profiler.takePreciseCoverage", None)
      .await?;
    let return_object = serde_json::from_value(return_value).map_err(|e| {
      InspectorPostMessageErrorKind::JsBox(JsErrorBox::from_err(e)).into_box()
    })?;
    Ok(return_object)
  }
}

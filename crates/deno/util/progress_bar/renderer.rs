use super::ProgressMessagePrompt;
use crate::util::display::human_download_size;
use deno_terminal::colors;
use std::fmt::Write;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
#[derive(Clone)]
pub struct ProgressDataDisplayEntry {
  pub prompt: ProgressMessagePrompt,
  pub message: String,
  pub position: u64,
  pub total_size: u64,
}
#[derive(Clone)]
pub struct ProgressData {
  pub terminal_width: u32,
  pub display_entries: Vec<ProgressDataDisplayEntry>,
  pub pending_entries: usize,
  pub percent_done: f64,
  pub total_entries: usize,
  pub duration: Duration,
}
pub trait ProgressBarRenderer: Send + Sync + std::fmt::Debug {
  fn render(&self, data: ProgressData) -> String;
}
/// Indicatif style progress bar.
#[derive(Debug)]
pub struct BarProgressBarRenderer {
  pub display_human_download_size: bool,
}
impl ProgressBarRenderer for BarProgressBarRenderer {
  fn render(&self, data: ProgressData) -> String {
    let Some(display_entry) = &data.display_entries.first() else {
      return String::new();
    };
    let (bytes_text, bytes_text_max_width) = {
      let total_size = display_entry.total_size;
      let pos = display_entry.position;
      if total_size == 0 {
        (String::new(), 0)
      } else {
        let (pos_str, total_size_str) = if self.display_human_download_size {
          (
            human_download_size(pos, total_size),
            human_download_size(total_size, total_size),
          )
        } else {
          (pos.to_string(), total_size.to_string())
        };
        (
          format!(" {}/{}", pos_str, total_size_str,),
          2 + total_size_str.len() * 2,
        )
      }
    };
    let (total_text, total_text_max_width) = if data.total_entries <= 1 {
      (String::new(), 0)
    } else {
      let total_entries_str = data.total_entries.to_string();
      (
        format!(
          " ({}/{})",
          data.total_entries - data.pending_entries,
          data.total_entries
        ),
        4 + total_entries_str.len() * 2,
      )
    };
    let elapsed_text = get_elapsed_text(data.duration);
    let mut text = String::new();
    if !display_entry.message.is_empty() {
      writeln!(
        &mut text,
        "{} {}{}",
        colors::green("Download"),
        display_entry.message,
        bytes_text,
      )
      .unwrap();
    }
    text.push_str(&elapsed_text);
    let max_width = (data.terminal_width as i32 - 5).clamp(10, 75) as usize;
    let same_line_text_width =
      elapsed_text.len() + total_text_max_width + bytes_text_max_width + 3;
    let total_bars = if same_line_text_width > max_width {
      1
    } else {
      max_width - same_line_text_width
    };
    let completed_bars =
      (total_bars as f64 * data.percent_done).floor() as usize;
    text.push_str(" [");
    if completed_bars != total_bars {
      if completed_bars > 0 {
        text.push_str(&format!(
          "{}",
          colors::cyan(format!("{}{}", "#".repeat(completed_bars - 1), ">"))
        ))
      }
      text.push_str(&format!(
        "{}",
        colors::intense_blue("-".repeat(total_bars - completed_bars))
      ))
    } else {
      text.push_str(&format!("{}", colors::cyan("#".repeat(completed_bars))))
    }
    text.push(']');
    if display_entry.message.is_empty() {
      text.push_str(&colors::gray(bytes_text).to_string());
    }
    text.push_str(&colors::gray(total_text).to_string());
    text
  }
}
#[derive(Debug)]
pub struct TextOnlyProgressBarRenderer {
  last_tick: AtomicUsize,
  start_time: std::time::Instant,
}
impl Default for TextOnlyProgressBarRenderer {
  fn default() -> Self {
    Self {
      last_tick: Default::default(),
      start_time: std::time::Instant::now(),
    }
  }
}
const SPINNER_CHARS: [&str; 13] = [
  "▰▱▱▱▱▱",
  "▰▰▱▱▱▱",
  "▰▰▰▱▱▱",
  "▰▰▰▰▱▱",
  "▰▰▰▰▰▱",
  "▰▰▰▰▰▰",
  "▰▰▰▰▰▰",
  "▱▰▰▰▰▰",
  "▱▱▰▰▰▰",
  "▱▱▱▰▰▰",
  "▱▱▱▱▰▰",
  "▱▱▱▱▱▰",
  "▱▱▱▱▱▱",
];
impl ProgressBarRenderer for TextOnlyProgressBarRenderer {
  fn render(&self, data: ProgressData) -> String {
    let last_tick = {
      let last_tick = self.last_tick.load(Ordering::Relaxed);
      let last_tick = (last_tick + 1) % SPINNER_CHARS.len();
      self.last_tick.store(last_tick, Ordering::Relaxed);
      last_tick
    };
    let current_time = std::time::Instant::now();
    let non_empty_entry = data
      .display_entries
      .iter()
      .find(|d| !d.message.is_empty() || d.total_size != 0);
    let prompt = match non_empty_entry {
      Some(entry) => entry.prompt,
      None => data.display_entries[0].prompt,
    };
    let mut display_str =
      format!("{} {} ", prompt.as_text(), SPINNER_CHARS[last_tick]);
    let elapsed_time = current_time - self.start_time;
    let fmt_elapsed_time = get_elapsed_text(elapsed_time);
    let total_text = if data.total_entries <= 1 {
      String::new()
    } else {
      format!(
        " {}/{}",
        data.total_entries - data.pending_entries,
        data.total_entries
      )
    };
    display_str.push_str(&format!("{}{}\n", fmt_elapsed_time, total_text));
    if let Some(display_entry) = non_empty_entry {
      let bytes_text = {
        let total_size = display_entry.total_size;
        let pos = display_entry.position;
        if total_size == 0 {
          String::new()
        } else {
          format!(
            " {}/{}",
            human_download_size(pos, total_size),
            human_download_size(total_size, total_size)
          )
        }
      };
      let message = display_entry
        .message
        .replace("https://registry.npmjs.org/", "npm:")
        .replace("https://jsr.io/", "jsr:")
        .replace("%2f", "/")
        .replace("%2F", "/");
      display_str.push_str(
        &colors::gray(format!("  {}{}\n", message, bytes_text)).to_string(),
      );
    } else {
      display_str.push('\n');
    }
    display_str
  }
}
fn get_elapsed_text(elapsed: Duration) -> String {
  let elapsed_secs = elapsed.as_secs();
  let seconds = elapsed_secs % 60;
  let minutes = elapsed_secs / 60;
  format!("[{minutes:0>2}:{seconds:0>2}]")
}

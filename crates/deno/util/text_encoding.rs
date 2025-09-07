use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use deno_core::ModuleSourceCode;
use std::ops::Range;
static SOURCE_MAP_PREFIX: &[u8] =
  b"//# sourceMappingURL=data:application/json;base64,";
pub fn source_map_from_code(code: &[u8]) -> Option<Vec<u8>> {
  let range = find_source_map_range(code)?;
  let source_map_range = &code[range];
  let input = source_map_range.split_at(SOURCE_MAP_PREFIX.len()).1;
  let decoded_map = BASE64_STANDARD.decode(input).ok()?;
  Some(decoded_map)
}
/// Truncate the source code before the source map.
pub fn code_without_source_map(code: ModuleSourceCode) -> ModuleSourceCode {
  use deno_core::ModuleCodeBytes;
  match code {
    ModuleSourceCode::String(mut code) => {
      if let Some(range) = find_source_map_range(code.as_bytes()) {
        code.truncate(range.start);
      }
      ModuleSourceCode::String(code)
    }
    ModuleSourceCode::Bytes(code) => {
      if let Some(range) = find_source_map_range(code.as_bytes()) {
        let source_map_index = range.start;
        ModuleSourceCode::Bytes(match code {
          ModuleCodeBytes::Static(bytes) => {
            ModuleCodeBytes::Static(&bytes[..source_map_index])
          }
          ModuleCodeBytes::Boxed(bytes) => ModuleCodeBytes::Boxed(
            bytes[..source_map_index].to_vec().into_boxed_slice(),
          ),
          ModuleCodeBytes::Arc(bytes) => ModuleCodeBytes::Boxed(
            bytes[..source_map_index].to_vec().into_boxed_slice(),
          ),
        })
      } else {
        ModuleSourceCode::Bytes(code)
      }
    }
  }
}
fn find_source_map_range(code: &[u8]) -> Option<Range<usize>> {
  fn last_non_blank_line_range(code: &[u8]) -> Option<Range<usize>> {
    let mut hit_non_whitespace = false;
    let mut range_end = code.len();
    for i in (0..code.len()).rev() {
      match code[i] {
        b' ' | b'\t' => {
          if !hit_non_whitespace {
            range_end = i;
          }
        }
        b'\n' | b'\r' => {
          if hit_non_whitespace {
            return Some(i + 1..range_end);
          }
          range_end = i;
        }
        _ => {
          hit_non_whitespace = true;
        }
      }
    }
    None
  }
  let range = last_non_blank_line_range(code)?;
  if code[range.start..range.end].starts_with(SOURCE_MAP_PREFIX) {
    Some(range)
  } else {
    None
  }
}

const BYTES_TO_KIB: u64 = 2u64.pow(10);
const BYTES_TO_MIB: u64 = 2u64.pow(20);
/// Gets the size used for downloading data. The total bytes is used to
/// determine the units to use.
pub fn human_download_size(byte_count: u64, total_bytes: u64) -> String {
  return if total_bytes < BYTES_TO_MIB {
    get_in_format(byte_count, BYTES_TO_KIB, "KiB")
  } else {
    get_in_format(byte_count, BYTES_TO_MIB, "MiB")
  };
  fn get_in_format(byte_count: u64, conversion: u64, suffix: &str) -> String {
    let converted_value = byte_count / conversion;
    let decimal = (byte_count % conversion) * 100 / conversion;
    format!("{converted_value}.{decimal:0>2}{suffix}")
  }
}

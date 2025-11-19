use std::io::{Error, ErrorKind};

/// Deserializes a snapshot from compressed data.
///
/// # Errors
///
/// Returns an error if:
/// - The data is too short (less than 4 bytes for size header)
/// - Decompression fails
pub fn deserialize_snapshot(data: &[u8]) -> std::io::Result<Box<[u8]>> {
  // compressing takes about a minute in debug, so only do it in release
  if cfg!(debug_assertions) {
    Ok(data.to_vec().into_boxed_slice())
  } else {
    if data.len() < 4 {
      return Err(Error::new(
        ErrorKind::InvalidData,
        "Snapshot data too short: expected at least 4 bytes for size header",
      ));
    }

    let size_bytes: [u8; 4] = data[0..4].try_into().expect("slice length verified above");
    let decompressed_size = u32::from_le_bytes(size_bytes) as usize;

    Ok(zstd::bulk::decompress(&data[4..], decompressed_size)?.into_boxed_slice())
  }
}

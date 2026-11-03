//! Safe, UI-independent Nitro file readers and lossless writers.
pub mod archive;
pub mod bmg;
pub mod detect;
pub mod graphics;
pub mod lz77;
pub mod rom;
pub mod texture;
pub use lz77::{compress, decompress, recompress};

#[derive(Debug, thiserror::Error)]
#[error("{message} at byte 0x{offset:x}")]
pub struct Error {
    pub offset: usize,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn error(offset: usize, message: impl Into<String>) -> Error {
    Error {
        offset,
        message: message.into(),
    }
}
pub(crate) fn slice(b: &[u8], at: usize, len: usize) -> Result<&[u8]> {
    b.get(
        at..at
            .checked_add(len)
            .ok_or_else(|| error(at, "range overflow"))?,
    )
    .ok_or_else(|| error(at, "truncated data"))
}
pub(crate) fn u16le(b: &[u8], at: usize) -> Result<u16> {
    let s = slice(b, at, 2)?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}
pub(crate) fn u32le(b: &[u8], at: usize) -> Result<u32> {
    let s = slice(b, at, 4)?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}
pub(crate) fn put16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
pub(crate) fn put32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

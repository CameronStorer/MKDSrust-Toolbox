//! Executable images retain the boot module parameters and optional tool footer.
use crate::{blz, error, u32le, Result};
pub const ARM9: &str = "system/arm9.bin";
pub fn overlay_path(id: u32) -> String {
    format!("system/overlay9/{id}.bin")
}
pub fn module_params(bytes: &[u8]) -> Option<usize> {
    bytes[..bytes.len().min(0x8000)]
        .windows(8)
        .position(|w| w == [0x21, 0x06, 0xc0, 0xde, 0xde, 0xc0, 0x06, 0x21])?
        .checked_sub(28)
}
pub fn decode_arm9(stored: &[u8]) -> Result<Vec<u8>> {
    let n = stored.len();
    let payload =
        if n >= 12 && stored[n - 12..n - 4] == [0xde, 0xc0, 0x06, 0x21, 0x21, 0x06, 0xc0, 0xde] {
            &stored[..n - 12]
        } else {
            stored
        };
    if let Some(p) = module_params(payload) {
        if u32le(payload, p + 20)? == 0 {
            return Ok(payload.to_vec());
        }
    }
    blz::decompress(payload)
}
pub fn encode_arm9(image: &[u8], base: u32, slot: usize, uncompressed: bool) -> Result<Vec<u8>> {
    let mut plain = image.to_vec();
    let p = module_params(&plain).ok_or_else(|| error(0, "ARM9 module parameters not found"))?;
    plain[p + 20..p + 24].fill(0);
    if !uncompressed {
        if let Some(first) = blz::compress(&plain, 0x4000.min(plain.len())) {
            let end = base
                .checked_add(first.len() as u32)
                .ok_or_else(|| error(0, "ARM9 RAM overflow"))?;
            plain[p + 20..p + 24].copy_from_slice(&end.to_le_bytes());
            if let Some(result) = blz::compress(&plain, 0x4000.min(plain.len())) {
                if result.len() == first.len() && result.len() <= slot {
                    return Ok(result);
                }
            }
            plain[p + 20..p + 24].fill(0);
        }
    }
    Ok(plain)
}

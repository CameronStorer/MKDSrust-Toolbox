//! BPS patches (the ROM-hacking standard by byuu/Near): create and apply.
//!
//! A patch checks the source ROM (CRC32), so applying it to the wrong ROM is refused instead of producing garbage.
use crate::{error, Result};

/// CRC-32 (IEEE, as BPS uses).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn encode(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let x = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(0x80 | x);
            break;
        }
        out.push(x);
        v -= 1;
    }
}

fn decode(p: &[u8], at: &mut usize) -> Result<u64> {
    let (mut data, mut shift) = (0u64, 1u64);
    loop {
        let x = *p
            .get(*at)
            .ok_or_else(|| error(*at, "truncated BPS number"))? as u64;
        *at += 1;
        data = data
            .checked_add(
                (x & 0x7F)
                    .checked_mul(shift)
                    .ok_or_else(|| error(*at, "BPS number overflow"))?,
            )
            .ok_or_else(|| error(*at, "BPS number overflow"))?;
        if x & 0x80 != 0 {
            return Ok(data);
        }
        shift = shift
            .checked_shl(7)
            .ok_or_else(|| error(*at, "BPS number overflow"))?;
        data = data
            .checked_add(shift)
            .ok_or_else(|| error(*at, "BPS number overflow"))?;
    }
}

const SOURCE_READ: u64 = 0;
const TARGET_READ: u64 = 1;
const SOURCE_COPY: u64 = 2;
const TARGET_COPY: u64 = 3;

/// A patch turning `source` into `target`. Linear encoding: bytes equal at the same offset are read from the source;
/// runs that differ are stored (good for rebuilt ROMs, where files change in place or move to the end).
pub fn create(source: &[u8], target: &[u8]) -> Vec<u8> {
    let mut out = b"BPS1".to_vec();
    encode(&mut out, source.len() as u64);
    encode(&mut out, target.len() as u64);
    encode(&mut out, 0);
    let same = |i: usize| i < source.len() && source[i] == target[i];
    let mut i = 0;
    while i < target.len() {
        let start = i;
        if same(i) {
            while i < target.len() && same(i) {
                i += 1;
            }
            encode(&mut out, ((i - start - 1) as u64) << 2 | SOURCE_READ);
        } else {
            // stop a literal run only at 8 equal bytes, so short matches don't cost more than they save
            while i < target.len() && !(0..8).all(|k| same(i + k) && i + k < target.len()) {
                i += 1;
            }
            encode(&mut out, ((i - start - 1) as u64) << 2 | TARGET_READ);
            out.extend_from_slice(&target[start..i]);
        }
    }
    out.extend_from_slice(&crc32(source).to_le_bytes());
    out.extend_from_slice(&crc32(target).to_le_bytes());
    let c = crc32(&out);
    out.extend_from_slice(&c.to_le_bytes());
    out
}

/// Applies a BPS patch; refuses a wrong source ROM or a damaged patch.
pub fn apply(source: &[u8], patch: &[u8]) -> Result<Vec<u8>> {
    if patch.len() < 4 + 12 || &patch[..4] != b"BPS1" {
        return Err(error(0, "not a BPS patch"));
    }
    let body = patch.len() - 12;
    let word = |at: usize| u32::from_le_bytes(patch[at..at + 4].try_into().unwrap());
    if crc32(&patch[..patch.len() - 4]) != word(patch.len() - 4) {
        return Err(error(
            patch.len() - 4,
            "BPS patch is damaged (checksum mismatch)",
        ));
    }
    if crc32(source) != word(body) {
        return Err(error(
            body,
            "this patch is for a different ROM (source checksum mismatch)",
        ));
    }
    let mut at = 4;
    let source_size = decode(patch, &mut at)? as usize;
    let target_size = decode(patch, &mut at)? as usize;
    let meta = decode(patch, &mut at)? as usize;
    at = at
        .checked_add(meta)
        .filter(|&a| a <= body)
        .ok_or_else(|| error(at, "bad BPS metadata"))?;
    if source_size != source.len() || target_size > 1 << 31 {
        return Err(error(4, "BPS sizes don't match the ROM"));
    }
    let mut out: Vec<u8> = Vec::with_capacity(target_size);
    let (mut src_rel, mut tgt_rel) = (0i64, 0i64);
    while at < body {
        let d = decode(patch, &mut at)?;
        let (mode, len) = (d & 3, (d >> 2) as usize + 1);
        if out.len() + len > target_size {
            return Err(error(at, "BPS action writes past the target"));
        }
        match mode {
            SOURCE_READ => {
                let s = out.len();
                out.extend_from_slice(
                    source
                        .get(s..s + len)
                        .ok_or_else(|| error(at, "SourceRead past the source"))?,
                );
            }
            TARGET_READ => {
                out.extend_from_slice(
                    patch
                        .get(at..at + len)
                        .filter(|_| at + len <= body)
                        .ok_or_else(|| error(at, "TargetRead past the patch"))?,
                );
                at += len;
            }
            SOURCE_COPY | TARGET_COPY => {
                let o = decode(patch, &mut at)?;
                let delta = (o >> 1) as i64 * if o & 1 != 0 { -1 } else { 1 };
                if mode == SOURCE_COPY {
                    src_rel += delta;
                    let s = usize::try_from(src_rel)
                        .ok()
                        .filter(|&s| s + len <= source.len())
                        .ok_or_else(|| error(at, "SourceCopy outside the source"))?;
                    out.extend_from_slice(&source[s..s + len]);
                    src_rel += len as i64;
                } else {
                    tgt_rel += delta;
                    let t = usize::try_from(tgt_rel)
                        .ok()
                        .filter(|&t| t < out.len())
                        .ok_or_else(|| error(at, "TargetCopy outside the target"))?;
                    // byte by byte: the copy may overlap what it is writing (a repeating pattern)
                    for k in t..t + len {
                        let b = out[k];
                        out.push(b);
                    }
                    tgt_rel += len as i64;
                }
            }
            _ => unreachable!(),
        }
    }
    if out.len() != target_size || crc32(&out) != word(body + 4) {
        return Err(error(body + 4, "patched result has the wrong checksum"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn patches_round_trip_and_refuse_wrong_sources() {
        let source: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();
        let mut target = source.clone();
        target[100..140].fill(0xAA);
        target[3000] ^= 1;
        target.extend_from_slice(&[1, 2, 3, 4, 5]);
        let p = create(&source, &target);
        assert!(p.len() < 200, "patch size {}", p.len());
        assert_eq!(apply(&source, &p).unwrap(), target);
        let mut wrong = source.clone();
        wrong[0] ^= 1;
        assert!(apply(&wrong, &p)
            .unwrap_err()
            .message
            .contains("different ROM"));
        let mut damaged = p.clone();
        let n = damaged.len();
        damaged[n / 2] ^= 1;
        assert!(apply(&source, &damaged).is_err());
        // shrinking and identical targets
        assert_eq!(
            apply(&source, &create(&source, &source[..10])).unwrap(),
            &source[..10]
        );
        assert_eq!(apply(&source, &create(&source, &source)).unwrap(), source);
    }

    #[test]
    fn applies_patches_with_copy_actions() {
        // hand-made: SourceCopy of source[2..4], then TargetCopy of the 2 bytes just written
        let source = b"abcdef";
        let mut p = b"BPS1".to_vec();
        encode(&mut p, 6);
        encode(&mut p, 4);
        encode(&mut p, 0);
        encode(&mut p, (1 << 2) | SOURCE_COPY);
        encode(&mut p, 2 << 1);
        encode(&mut p, (1 << 2) | TARGET_COPY);
        encode(&mut p, 0);
        p.extend_from_slice(&crc32(source).to_le_bytes());
        p.extend_from_slice(&crc32(b"cdcd").to_le_bytes());
        let c = crc32(&p);
        p.extend_from_slice(&c.to_le_bytes());
        assert_eq!(apply(source, &p).unwrap(), b"cdcd");
    }
}

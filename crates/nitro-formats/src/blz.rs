//! Backwards LZ used by DS executable images. All offsets are checked before decoding.
use crate::{error, u32le, Result};
use std::collections::HashMap;

pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let n = data.len();
    if n < 8 {
        return Err(error(0, "truncated BLZ footer"));
    }
    let footer = u32le(data, n - 8)?;
    let extra = u32le(data, n - 4)? as usize;
    if extra == 0 {
        return Ok(data.to_vec());
    }
    let header = (footer >> 24) as usize;
    let encoded = (footer & 0xffffff) as usize;
    if header < 8
        || encoded < header
        || encoded > n
        || n.checked_add(extra).is_none_or(|v| v > 16 * 1024 * 1024)
    {
        return Err(error(n - 8, "invalid BLZ lengths"));
    }
    let stop = n - encoded;
    let mut read = n - header;
    let mut write = n + extra;
    let mut out = vec![0; write];
    out[..n].copy_from_slice(data);
    while read > stop {
        read -= 1;
        let flags = data[read];
        for bit in 0..8 {
            if read == stop {
                break;
            }
            if flags & (0x80 >> bit) == 0 {
                if write <= stop {
                    return Err(error(read, "BLZ output underflow"));
                }
                read -= 1;
                write -= 1;
                out[write] = data[read];
            } else {
                if read < stop + 2 {
                    return Err(error(read, "truncated BLZ match"));
                }
                read -= 2;
                let a = data[read + 1] as usize;
                let count = (a >> 4) + 3;
                let distance = ((a & 15) << 8 | data[read] as usize) + 3;
                if write < stop + count || write.checked_add(distance).is_none_or(|v| v > out.len())
                {
                    return Err(error(read, "invalid BLZ backreference"));
                }
                for _ in 0..count {
                    write -= 1;
                    out[write] = out[write + distance];
                }
            }
            if write < read {
                return Err(error(read, "BLZ overwrites unread input"));
            }
        }
    }
    if write != stop {
        return Err(error(read, "BLZ output length mismatch"));
    }
    Ok(out)
}

/// Compress a suffix, retaining at least `prefix` bytes for the boot decompressor.
/// None means storing the image uncompressed is smaller.
pub fn compress(data: &[u8], prefix: usize) -> Option<Vec<u8>> {
    if data.len() > 16 * 1024 * 1024 || prefix > data.len() {
        return None;
    }
    let rev: Vec<_> = data.iter().rev().copied().collect();
    let limit = data.len() - prefix;
    let mut chains: HashMap<[u8; 3], Vec<usize>> = HashMap::new();
    let mut stream = vec![];
    let mut pos = 0;
    let mut best = (0, 0, 0);
    while pos < limit {
        let flag_at = stream.len();
        stream.push(0);
        for bit in 0..8 {
            if pos >= limit {
                break;
            }
            let mut length = 0;
            let mut distance = 0;
            if pos + 3 <= limit {
                let key = [rev[pos], rev[pos + 1], rev[pos + 2]];
                if let Some(candidates) = chains.get(&key) {
                    for &old in candidates.iter().rev().take(128) {
                        let d = pos - old;
                        if d > 4098 {
                            break;
                        }
                        if d < 3 {
                            continue;
                        }
                        let mut len = 3;
                        while len < 18 && pos + len < limit && rev[pos + len] == rev[pos + len - d]
                        {
                            len += 1;
                        }
                        if len > length {
                            length = len;
                            distance = d;
                        }
                    }
                }
            }
            let count = if length >= 3 {
                stream[flag_at] |= 0x80 >> bit;
                stream.push((((length - 3) << 4) | ((distance - 3) >> 8)) as u8);
                stream.push((distance - 3) as u8);
                length
            } else {
                stream.push(rev[pos]);
                1
            };
            for p in pos..pos + count {
                if p + 3 <= limit {
                    let entries = chains.entry([rev[p], rev[p + 1], rev[p + 2]]).or_default();
                    if entries.len() >= 256 {
                        entries.drain(..128);
                    }
                    entries.push(p);
                }
            }
            pos += count;
        }
        let padding = (4 - (data.len() - pos + stream.len()) % 4) % 4;
        let cost = stream.len() + padding + 8;
        if pos > cost && pos - cost > best.0 {
            best = (pos - cost, pos, stream.len());
        }
    }
    if best.0 == 0 {
        return None;
    }
    let mut out = data[..data.len() - best.1].to_vec();
    out.extend(stream[..best.2].iter().rev());
    let padding = (4 - out.len() % 4) % 4;
    out.resize(out.len() + padding, 0xff);
    let encoded = best.2 + padding + 8;
    out.extend_from_slice(&((encoded as u32) | ((padding as u32 + 8) << 24)).to_le_bytes());
    out.extend_from_slice(&((data.len() - out.len() - 4) as u32).to_le_bytes());
    if decompress(&out).ok().as_deref() == Some(data) {
        Some(out)
    } else {
        None
    }
}

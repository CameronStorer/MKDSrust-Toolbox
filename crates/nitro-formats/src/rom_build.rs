//! Rebuilding a ROM with changed files, for real hardware (ndstool's job, done conservatively).
//!
//! A changed file is written in place when it fits the space before the next section, otherwise it moves to the
//! end of the ROM (0x200-aligned). Every other byte stays as it was, so patches stay small and code, overlays and
//! tables don't move. The header's used size, chip capacity and header CRC are updated (real hardware and
//! flashcarts check the CRC).
use crate::{error, rom::Rom, u32le, Result};
use std::collections::BTreeMap;

/// Padding byte for freed or added space (what cartridge images use).
const PAD: u8 = 0xFF;
/// Alignment of files moved to the end.
const ALIGN: usize = 0x200;

/// CRC-16 (MODBUS/ARC, as the DS header and banner checksums use).
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &b in data {
        crc ^= b as u16;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

/// The header CRC (0x15E) over bytes 0x000-0x15D.
pub fn fix_header_crc(rom: &mut [u8]) {
    let c = crc16(&rom[..0x15E]);
    rom[0x15E..0x160].copy_from_slice(&c.to_le_bytes());
}

fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// What a rebuild did, for the user and for checks.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BuildReport {
    /// Files written where they were.
    pub in_place: Vec<String>,
    /// Files moved to the end of the ROM.
    pub moved: Vec<String>,
    /// The new ROM size in bytes.
    pub size: usize,
}

/// `original` with the files in `changes` (by ROM path) replaced.
pub fn rebuild(
    original: &[u8],
    changes: &BTreeMap<String, Vec<u8>>,
) -> Result<(Vec<u8>, BuildReport)> {
    // Keep the ARM9 secure area at its original cartridge address. Make room by
    // shifting later sections, including unnamed overlay FAT entries.
    if let Some(arm9) = changes.get(crate::executable::ARM9) {
        let old = Rom::parse(original)?.arm9;
        if arm9.len() > old.len() {
            let added = (arm9.len() - old.len()).div_ceil(ALIGN) * ALIGN;
            let mut expanded = Vec::with_capacity(original.len() + added);
            expanded.extend_from_slice(&original[..old.end]);
            expanded.resize(old.end + added, PAD);
            expanded.extend_from_slice(&original[old.end..]);
            for field in [0x30, 0x40, 0x48, 0x50, 0x58, 0x68] {
                let value = u32le(original, field)? as usize;
                if value >= old.end {
                    put(&mut expanded, field, (value + added) as u32);
                }
            }
            let fat = u32le(&expanded, 0x48)? as usize;
            let count = u32le(&expanded, 0x4c)? as usize / 8;
            for i in 0..count {
                for at in [fat + i * 8, fat + i * 8 + 4] {
                    let value = u32le(&expanded, at)? as usize;
                    if value >= old.end {
                        put(&mut expanded, at, (value + added) as u32);
                    }
                }
            }
            let used = u32le(original, 0x80)? as usize;
            if used >= old.end {
                put(&mut expanded, 0x80, (used + added) as u32);
            }
            // The recursive pass sees a reserved slot and cannot expand again.
            put(&mut expanded, 0x2c, (old.len() + added) as u32);
            return rebuild(&expanded, changes);
        }
    }
    let rom = Rom::parse(original)?;
    let mut out = original.to_vec();
    let fat_at = u32le(original, 0x48)? as usize;
    let fat_len = u32le(original, 0x4c)? as usize;
    // every region a file must not run into: all FAT entries plus the header-listed sections
    let mut starts: Vec<usize> = (0..fat_len / 8)
        .map(|i| u32le(original, fat_at + i * 8))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|v| v as usize)
        .collect();
    for (at, _len) in [
        (0x20, 0x2c),
        (0x30, 0x3c),
        (0x40, 0x44),
        (0x48, 0x4c),
        (0x50, 0x54),
        (0x58, 0x5c),
    ] {
        starts.push(u32le(original, at)? as usize);
    }
    let banner = u32le(original, 0x68)? as usize;
    if banner != 0 {
        starts.push(banner);
    }
    starts.sort_unstable();
    let used = u32le(original, 0x80)? as usize;
    let data_end = rom
        .files
        .values()
        .map(|r| r.end)
        .max()
        .unwrap_or(0)
        .max(used)
        .max(banner + 0x840 * (banner != 0) as usize);
    // appended files go after everything the game uses, not after trailing padding the dump may have
    let mut tail = data_end.div_ceil(ALIGN) * ALIGN;
    let mut report = BuildReport::default();
    let mut section_end = used;
    for (path, data) in changes {
        let special = if path == crate::executable::ARM9 {
            Some((rom.arm9.clone(), None))
        } else if let Some(id) = path
            .strip_prefix("system/overlay9/")
            .and_then(|v| v.strip_suffix(".bin"))
            .and_then(|v| v.parse::<u32>().ok())
        {
            let (index, overlay) = rom
                .arm9_overlays
                .iter()
                .enumerate()
                .find(|(_, o)| o.id == id)
                .ok_or_else(|| error(0, "unknown ARM9 overlay"))?;
            let at = fat_at + overlay.file_id as usize * 8;
            Some((
                u32le(original, at)? as usize..u32le(original, at + 4)? as usize,
                Some((index, overlay)),
            ))
        } else {
            None
        };
        if let Some((old, overlay)) = special {
            let compressed = overlay
                .and_then(|_| crate::blz::compress(data, 0))
                .filter(|b| b.len() <= old.len());
            let stored = compressed.as_deref().unwrap_or(data);
            let next = starts
                .iter()
                .copied()
                .filter(|&s| s > old.start)
                .min()
                .unwrap_or(old.end);
            let at = if stored.len() <= next - old.start {
                report.in_place.push(path.clone());
                old.start
            } else {
                report.moved.push(path.clone());
                out[old.clone()].fill(PAD);
                let at = tail;
                tail = (at + stored.len()).div_ceil(ALIGN) * ALIGN;
                at
            };
            if out.len() < at + stored.len() {
                out.resize(at + stored.len(), PAD);
            }
            out[at..at + stored.len()].copy_from_slice(stored);
            section_end = section_end.max(at + stored.len());
            if let Some((index, o)) = overlay {
                let fat = fat_at + o.file_id as usize * 8;
                put(&mut out, fat, at as u32);
                put(&mut out, fat + 4, (at + stored.len()) as u32);
                let table = u32le(original, 0x50)? as usize + index * 32;
                if data.len() != o.ram_size as usize {
                    return Err(error(table, "overlay loaded size changed"));
                }
                // A modified overlay cannot retain the original hash-verification flag.
                put(
                    &mut out,
                    table + 28,
                    (o.flags & 0xfc00_0000)
                        | if compressed.is_some() {
                            0x0100_0000 | stored.len() as u32
                        } else {
                            0
                        },
                );
            } else {
                put(&mut out, 0x20, at as u32);
                put(&mut out, 0x2c, data.len() as u32);
            }
            continue;
        }
        let id = *rom
            .file_ids
            .get(path)
            .ok_or_else(|| error(0, format!("no file {path} in the ROM")))?;
        let old = rom.files[path].clone();
        let next = starts
            .iter()
            .copied()
            .filter(|&s| s > old.start)
            .min()
            .unwrap_or(data_end.max(old.end));
        let room = next - old.start;
        let at = if data.len() <= room {
            report.in_place.push(path.clone());
            old.start
        } else {
            report.moved.push(path.clone());
            // the old space is freed (padding), the file goes to the end
            out[old.clone()].fill(PAD);
            let at = tail;
            tail = (at + data.len()).div_ceil(ALIGN) * ALIGN;
            at
        };
        if out.len() < at + data.len() {
            out.resize(at + data.len(), PAD);
        }
        out[at..at + data.len()].copy_from_slice(data);
        if at == old.start && data.len() < old.len() {
            out[at + data.len()..old.end].fill(PAD);
        }
        put(&mut out, fat_at + id * 8, at as u32);
        put(&mut out, fat_at + id * 8 + 4, (at + data.len()) as u32);
    }
    // used size, then the chip capacity (128 KiB << n), which must hold the whole image
    let new_used = (0..fat_len / 8)
        .map(|i| u32le(&out, fat_at + i * 8 + 4).map(|v| v as usize))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max()
        .unwrap_or(0)
        .max(section_end);
    // only grow the used-size field when something was placed past it (an unchanged build stays byte-identical)
    if (!report.moved.is_empty() || section_end > used) && new_used > used {
        put(&mut out, 0x80, new_used as u32);
    }
    let mut cap = out[0x14];
    while (0x20000usize << cap) < out.len() && cap < 15 {
        cap += 1;
    }
    out[0x14] = cap;
    fix_header_crc(&mut out);
    report.size = out.len();
    // the result must read back with every changed file intact
    let check = Rom::parse(&out)?;
    for (path, data) in changes {
        let actual = if path == crate::executable::ARM9 {
            Some(&out[check.arm9.clone()])
        } else if path.starts_with("system/overlay9/") {
            let id = path
                .trim_start_matches("system/overlay9/")
                .trim_end_matches(".bin")
                .parse::<u32>()
                .ok();
            check
                .arm9_overlays
                .iter()
                .find(|o| Some(o.id) == id)
                .and_then(|o| check.file_by_id(o.file_id).ok())
        } else {
            check.file(path)
        };
        let decoded = if path.starts_with("system/overlay9/") {
            actual
                .map(|b| {
                    if b.len() == data.len() {
                        Ok(b.to_vec())
                    } else {
                        crate::blz::decompress(b)
                    }
                })
                .transpose()?
        } else {
            None
        };
        if decoded.as_deref().or(actual) != Some(data.as_slice()) {
            return Err(error(0, format!("rebuilt ROM does not read back {path}")));
        }
    }
    Ok((out, report))
}

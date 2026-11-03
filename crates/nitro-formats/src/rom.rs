use crate::{archive::filenames, error, slice, u16le, u32le, Result};
use std::{collections::BTreeMap, ops::Range};

#[derive(Clone, Debug)]
pub struct Overlay {
    pub id: u32,
    pub ram_address: u32,
    pub ram_size: u32,
    pub bss_size: u32,
    pub static_start: u32,
    pub static_end: u32,
    pub file_id: u32,
    pub flags: u32,
}
#[derive(Clone, Debug)]
pub struct Banner {
    pub version: u16,
    pub icon: Vec<u8>,
    pub palette: Vec<u16>,
    pub titles: Vec<Vec<u16>>,
}
#[derive(Clone, Debug)]
pub struct Rom {
    bytes: Vec<u8>,
    pub game_code: String,
    pub revision: u8,
    pub title: String,
    pub files: BTreeMap<String, Range<usize>>,
    pub arm9: Range<usize>,
    pub arm7: Range<usize>,
    pub arm9_overlays: Vec<Overlay>,
    pub arm7_overlays: Vec<Overlay>,
    pub banner: Option<Banner>,
}
impl Rom {
    pub fn parse(b: &[u8]) -> Result<Self> {
        slice(b, 0, 512)?;
        let range = |at, len_at| -> Result<Range<usize>> {
            let start = u32le(b, at)? as usize;
            let len = u32le(b, len_at)? as usize;
            if len > 0 && start < 512 {
                return Err(error(at, "section overlaps header"));
            }
            slice(b, start, len)?;
            Ok(start..start + len)
        };
        let fnt = range(0x40, 0x44)?;
        let fat = range(0x48, 0x4c)?;
        if fat.len() % 8 != 0 {
            return Err(error(0x4c, "FAT size is not a multiple of 8"));
        }
        let count = fat.len() / 8;
        let names = filenames(&b[fnt.clone()], count)
            .map_err(|e| error(fnt.start + e.offset, e.message))?;
        let mut files = BTreeMap::new();
        let mut ranges = vec![];
        for i in 0..count {
            let a = u32le(b, fat.start + i * 8)? as usize;
            let z = u32le(b, fat.start + i * 8 + 4)? as usize;
            if a < 512 || z < a {
                return Err(error(fat.start + i * 8, "invalid FAT range"));
            }
            slice(b, a, z - a)?;
            ranges.push(a..z);
        }
        for (i, name) in names.into_iter().enumerate() {
            if let Some(name) = name {
                files.insert(name, ranges[i].clone());
            }
        }
        let overlays = |r: Range<usize>| -> Result<Vec<Overlay>> {
            if !r.len().is_multiple_of(32) {
                return Err(error(r.start, "invalid overlay table length"));
            }
            let mut out = vec![];
            for p in (r.start..r.end).step_by(32) {
                let file_id = u32le(b, p + 24)?;
                if file_id as usize >= count {
                    return Err(error(p + 24, "overlay file id outside FAT"));
                }
                out.push(Overlay {
                    id: u32le(b, p)?,
                    ram_address: u32le(b, p + 4)?,
                    ram_size: u32le(b, p + 8)?,
                    bss_size: u32le(b, p + 12)?,
                    static_start: u32le(b, p + 16)?,
                    static_end: u32le(b, p + 20)?,
                    file_id,
                    flags: u32le(b, p + 28)?,
                });
            }
            Ok(out)
        };
        let at = u32le(b, 0x68)? as usize;
        let banner = if at == 0 {
            None
        } else {
            let version = u16le(b, at)?;
            let languages = match version {
                1 => 6,
                2 => 7,
                3 | 0x103 => 8,
                _ => return Err(error(at, "unsupported banner version")),
            };
            let icon = slice(b, at + 0x20, 512)?.to_vec();
            let palette = (0..16)
                .map(|i| u16le(b, at + 0x220 + i * 2))
                .collect::<Result<_>>()?;
            let titles = (0..languages)
                .map(|n| {
                    (0..128)
                        .map(|i| u16le(b, at + 0x240 + n * 256 + i * 2))
                        .collect::<Result<_>>()
                })
                .collect::<Result<_>>()?;
            Some(Banner {
                version,
                icon,
                palette,
                titles,
            })
        };
        Ok(Self {
            bytes: b.to_vec(),
            game_code: String::from_utf8_lossy(&b[12..16]).into(),
            revision: b[0x1e],
            title: String::from_utf8_lossy(&b[..12])
                .trim_end_matches('\0')
                .into(),
            files,
            arm9: range(0x20, 0x2c)?,
            arm7: range(0x30, 0x3c)?,
            arm9_overlays: overlays(range(0x50, 0x54)?)?,
            arm7_overlays: overlays(range(0x58, 0x5c)?)?,
            banner,
        })
    }
    pub fn file(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(|r| &self.bytes[r.clone()])
    }
    /// Read-only image, useful for hashing without extracting.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

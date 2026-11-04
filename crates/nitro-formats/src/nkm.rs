//! NKMD layouts independently implemented from Mario Kart Toolbox's MIT format definitions.
//! Records retain every fixed-point bit, unknown field, padding byte and section order.
use crate::{error, put32, slice, u16le, u32le, Result};
#[derive(Clone, Debug)]
pub struct Section {
    pub kind: String,
    pub entries: Vec<Vec<u8>>,
    at: usize,
    stride: usize,
}
#[derive(Clone, Debug)]
pub struct Nkm {
    pub version: u16,
    pub sections: Vec<Section>,
    raw: Vec<u8>,
}
pub fn stride(kind: &str, version: u16) -> Option<usize> {
    Some(match kind {
        "OBJI" => 60,
        "PATH" => 4,
        "POIT" => 20,
        "STAG" => 40,
        "KTPS" | "KTP2" | "KTPC" | "KTPM" => 28,
        "KTPJ" => {
            if version >= 32 {
                32
            } else {
                28
            }
        }
        "CPOI" => 36,
        "CPAT" | "IPAT" | "EPAT" => 12,
        "IPOI" => {
            if version >= 34 {
                20
            } else {
                16
            }
        }
        "EPOI" | "MEPO" => 24,
        "MEPA" => 20,
        "AREA" => 72,
        "CAME" => 76,
        _ => return None,
    })
}
impl Nkm {
    pub fn parse(b: &[u8]) -> Result<Self> {
        if slice(b, 0, 4)? != b"NKMD" {
            return Err(error(0, "expected NKMD"));
        }
        let version = u16le(b, 4)?;
        let h = u16le(b, 6)? as usize;
        if h < 8 || !(h - 8).is_multiple_of(4) {
            return Err(error(6, "invalid NKM header size"));
        }
        slice(b, 0, h)?;
        let offsets = (0..(h - 8) / 4)
            .map(|i| Ok(h + u32le(b, 8 + i * 4)? as usize))
            .collect::<Result<Vec<_>>>()?;
        let mut sections = vec![];
        for &at in &offsets {
            let kind = String::from_utf8_lossy(slice(b, at, 4)?).into_owned();
            let stride = stride(&kind, version).ok_or_else(|| error(at, "unknown NKM section"))?;
            let count = if kind == "STAG" {
                1
            } else {
                u32le(b, at + 4)? as usize
            };
            let start = at + if kind == "STAG" { 4 } else { 8 };
            let len = count
                .checked_mul(stride)
                .ok_or_else(|| error(at, "NKM count overflow"))?;
            let end = offsets
                .iter()
                .copied()
                .filter(|o| *o > at)
                .min()
                .unwrap_or(b.len());
            if start.checked_add(len).is_none_or(|p| p > end) {
                return Err(error(at, "NKM entries overlap next section"));
            }
            if offsets.iter().filter(|o| **o == at).count() != 1 {
                return Err(error(at, "duplicate NKM section offset"));
            }
            let entries = slice(b, start, len)?
                .chunks_exact(stride)
                .map(|b| b.to_vec())
                .collect();
            sections.push(Section {
                kind,
                entries,
                at: start,
                stride,
            });
        }
        Ok(Self {
            version,
            sections,
            raw: b.to_vec(),
        })
    }
    /// Fixed-size replacement records are writable without losing noncanonical layout/padding.
    pub fn write(&self) -> Result<Vec<u8>> {
        let original = Self::parse(&self.raw)?;
        if original.version != self.version
            || original.sections.len() != self.sections.len()
            || original.sections.iter().zip(&self.sections).any(|(a, b)| {
                a.kind != b.kind || a.entries.len() != b.entries.len() || a.at != b.at
            })
        {
            return Self::new_version(
                self.version,
                &self
                    .sections
                    .iter()
                    .map(|s| (s.kind.as_str(), s.entries.clone()))
                    .collect::<Vec<_>>(),
            )?
            .write();
        }
        let mut b = self.raw.clone();
        for s in &self.sections {
            for (i, e) in s.entries.iter().enumerate() {
                if e.len() != s.stride {
                    return Err(error(s.at, "NKM record size changed"));
                }
                let at = s.at + i * s.stride;
                slice(&b, at, e.len())?;
                b[at..at + e.len()].copy_from_slice(e)
            }
        }
        Ok(b)
    }
    pub fn section(&self, kind: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.kind == kind)
    }
    /// Construct synthetic or author-owned records in the current (v37) layout.
    pub fn new(sections: &[(&str, Vec<Vec<u8>>)]) -> Result<Self> {
        Self::new_version(37, sections)
    }
    pub fn new_version(version: u16, sections: &[(&str, Vec<Vec<u8>>)]) -> Result<Self> {
        let h = 8 + sections.len() * 4;
        if h > u16::MAX as usize {
            return Err(error(0, "too many NKM sections"));
        }
        let mut b = vec![0; h];
        b[..4].copy_from_slice(b"NKMD");
        b[4..6].copy_from_slice(&version.to_le_bytes());
        b[6..8].copy_from_slice(&(h as u16).to_le_bytes());
        for (i, (kind, entries)) in sections.iter().enumerate() {
            if kind.len() != 4 {
                return Err(error(0, "invalid section name"));
            }
            let size = stride(kind, version).ok_or_else(|| error(0, "unknown NKM section"))?;
            if entries.iter().any(|e| e.len() != size) || (*kind == "STAG" && entries.len() != 1) {
                return Err(error(0, "invalid NKM record size/count"));
            }
            let offset = (b.len() - h) as u32;
            put32(&mut b, 8 + i * 4, offset);
            b.extend_from_slice(kind.as_bytes());
            if *kind != "STAG" {
                b.extend_from_slice(&(entries.len() as u32).to_le_bytes())
            }
            for e in entries {
                b.extend_from_slice(e)
            }
        }
        Self::parse(&b)
    }
}
pub fn fx(b: &[u8], at: usize) -> Result<f32> {
    Ok(u32le(b, at)? as i32 as f32 / 4096.)
}
pub fn vec3(b: &[u8], at: usize) -> Result<[f32; 3]> {
    Ok([fx(b, at)?, fx(b, at + 4)?, fx(b, at + 8)?])
}

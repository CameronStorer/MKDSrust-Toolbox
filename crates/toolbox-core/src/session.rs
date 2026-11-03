use crate::{Error, Result};
use nitro_formats::{archive::Narc, detect, rom::Rom};
use serde::Serialize;
use sha2::{Digest, Sha256};
use specta::Type;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub fn hash(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct FileNode {
    pub path: String,
    pub name: String,
    pub format: String,
    pub category: String,
    pub size: u32,
    pub offset: u32,
    pub compressed: bool,
    pub archive: bool,
    pub parent: Option<String>,
}
pub struct RomSession {
    pub rom: Rom,
    pub path: PathBuf,
    pub sha256: String,
    pub nodes: Vec<FileNode>,
    pub warnings: Vec<String>,
    files: BTreeMap<String, Vec<u8>>,
}
impl RomSession {
    pub fn open(path: &Path) -> Result<Self> {
        let path = path.canonicalize()?;
        Self::from_bytes(std::fs::read(&path)?, path)
    }
    pub fn from_bytes(bytes: Vec<u8>, path: PathBuf) -> Result<Self> {
        let sha256 = hash(&bytes);
        let rom = Rom::parse(&bytes)?;
        let mut warnings = vec![];
        if rom.game_code != mkds_data::GAME_CODE || rom.revision != 0 {
            warnings.push(format!(
                "ROM is {} revision {}; packages require AMCE revision 0",
                rom.game_code, rom.revision
            ));
        }
        let mut s = Self {
            rom,
            path,
            sha256,
            nodes: vec![],
            warnings,
            files: BTreeMap::new(),
        };
        let paths = s.rom.files.keys().cloned().collect::<Vec<_>>();
        let mut budget = 256 * 1024 * 1024;
        for path in paths {
            let range = s.rom.files[&path].clone();
            let bytes = s.rom.file(&path).unwrap_or_default().to_vec();
            s.visit(path, bytes, range.start, None, 0, &mut budget)?;
        }
        Ok(s)
    }
    fn visit(
        &mut self,
        path: String,
        bytes: Vec<u8>,
        offset: usize,
        parent: Option<String>,
        depth: usize,
        budget: &mut usize,
    ) -> Result<()> {
        if bytes.len() > *budget {
            self.warnings
                .push(format!("{path}: archive expansion budget exhausted"));
            return Ok(());
        }
        *budget -= bytes.len();
        let f = detect::detect(&bytes, &path);
        let archive = f.as_ref().is_some_and(|f| f.category == "Archive");
        self.nodes.push(FileNode {
            path: path.clone(),
            name: path.rsplit('/').next().unwrap_or(&path).into(),
            format: f.as_ref().map_or("bin", |f| f.extension).into(),
            category: f.as_ref().map_or("Binary", |f| f.category).into(),
            size: bytes.len() as u32,
            offset: offset as u32,
            compressed: f.as_ref().is_some_and(|f| f.compressed),
            archive,
            parent,
        });
        if archive {
            if depth >= 8 {
                self.warnings
                    .push(format!("{path}: archives nested more than 8 levels"));
            } else {
                let result = (|| -> Result<Narc> {
                    let plain = if bytes.first() == Some(&0x10) {
                        nitro_formats::decompress(&bytes)?
                    } else {
                        bytes.clone()
                    };
                    Ok(Narc::parse(&plain)?)
                })();
                match result {
                    Ok(narc) => {
                        for e in narc.entries {
                            self.visit(
                                format!("{path}/{}", e.name),
                                e.data,
                                e.offset,
                                Some(path.clone()),
                                depth + 1,
                                budget,
                            )?;
                        }
                    }
                    Err(e) => self.warnings.push(format!("{path}: {e}")),
                }
            }
        }
        self.files.insert(path, bytes);
        Ok(())
    }
    pub fn file(&self, path: &str) -> Result<&[u8]> {
        self.files
            .get(path)
            .map(Vec::as_slice)
            .or_else(|| self.rom.file(path))
            .ok_or_else(|| Error::Invalid(format!("file not found: {path}")))
    }
    pub fn root_for<'a>(&'a self, path: &str) -> Option<&'a str> {
        self.rom
            .files
            .keys()
            .filter(|p| path == p.as_str() || path.starts_with(&format!("{p}/")))
            .max_by_key(|p| p.len())
            .map(String::as_str)
    }
}

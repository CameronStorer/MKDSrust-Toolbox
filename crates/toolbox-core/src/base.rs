//! Imported hacks remain patches against the clean ROM, with provenance and a working in-memory image.
use crate::{
    project::{atomic_write, ensure_no_symlinks, Manifest, Project, ProjectConfig},
    session::{hash, RomSession},
    Error, Result,
};
use nitro_formats::{bps, rom::Rom};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct BaseConfig {
    pub clean_sha256: String,
    pub patch: String,
    pub source_name: String,
    pub source_sha256: String,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct BaseChange {
    pub path: String,
    pub change: String,
}

pub fn changes(session: &RomSession) -> Result<Vec<BaseChange>> {
    if session.clean.is_none() {
        return Ok(vec![]);
    }
    let clean = Rom::parse(session.clean_bytes())?;
    let mut result = vec![];
    let paths: BTreeSet<_> = clean.files.keys().chain(session.rom.files.keys()).collect();
    for path in paths {
        let (a, b) = (clean.file(path), session.rom.file(path));
        if a != b {
            result.push(BaseChange {
                path: path.clone(),
                change: match (a, b) {
                    (None, _) => "added",
                    (_, None) => "removed",
                    _ => "changed",
                }
                .into(),
            });
        }
    }
    for (name, a, b) in [
        ("arm9", clean.arm9.clone(), session.rom.arm9.clone()),
        ("arm7", clean.arm7.clone(), session.rom.arm7.clone()),
    ] {
        if clean.bytes()[a] != session.rom.bytes()[b] {
            result.push(BaseChange {
                path: name.into(),
                change: "changed code".into(),
            });
        }
    }
    for (name, a, b) in [
        (
            "ARM9 overlays",
            &clean.arm9_overlays,
            &session.rom.arm9_overlays,
        ),
        (
            "ARM7 overlays",
            &clean.arm7_overlays,
            &session.rom.arm7_overlays,
        ),
    ] {
        let overlay_bytes =
            |rom: &Rom, overlays: &[nitro_formats::rom::Overlay]| -> Result<Vec<Vec<u8>>> {
                overlays
                    .iter()
                    .map(|o| Ok(rom.file_by_id(o.file_id)?.to_vec()))
                    .collect()
            };
        if format!("{a:?}") != format!("{b:?}")
            || overlay_bytes(&clean, a)? != overlay_bytes(&session.rom, b)?
        {
            result.push(BaseChange {
                path: name.into(),
                change: "changed code".into(),
            });
        }
    }
    Ok(result)
}
pub fn create(
    root: &Path,
    clean: &RomSession,
    manifest: Manifest,
    source: &Path,
) -> Result<(Project, RomSession)> {
    if root.exists() && source.canonicalize()?.starts_with(root.canonicalize()?) {
        return Err(Error::Invalid(
            "keep the imported ROM or patch outside the project folder".into(),
        ));
    }
    let bytes = std::fs::read(source)?;
    let patch = if bytes.starts_with(b"BPS1") {
        bytes.clone()
    } else {
        Rom::parse(&bytes)?;
        bps::create(clean.clean_bytes(), &bytes)
    };
    let session = clean.with_base(&patch)?;
    if session.rom.game_code != clean.rom.game_code || session.rom.revision != clean.rom.revision {
        return Err(Error::Invalid(
            "base targets a different game or revision".into(),
        ));
    }
    let mut project = Project::create(root, clean, manifest)?;
    project.config.base = Some(BaseConfig {
        clean_sha256: clean.sha256.clone(),
        patch: "base.bps".into(),
        source_name: source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        source_sha256: hash(&bytes),
    });
    atomic_write(&project.root.join("base.bps"), &patch)?;
    atomic_write(
        &project.root.join("project.toml"),
        toml::to_string_pretty(&project.config)?.as_bytes(),
    )?;
    Ok((project, session))
}
pub fn working(root: &Path, clean: &RomSession) -> Result<RomSession> {
    let config: ProjectConfig =
        toml::from_str(&std::fs::read_to_string(root.join("project.toml"))?)?;
    if let Some(base) = config.base {
        if base.clean_sha256 != clean.sha256 || base.patch != "base.bps" {
            return Err(Error::Invalid(
                "invalid project base or clean ROM hash".into(),
            ));
        }
        ensure_no_symlinks(root, &root.join(&base.patch))?;
        clean.with_base(&std::fs::read(root.join(base.patch))?)
    } else {
        RomSession::from_bytes(clean.clean_bytes().to_vec(), clean.path.clone())
    }
}

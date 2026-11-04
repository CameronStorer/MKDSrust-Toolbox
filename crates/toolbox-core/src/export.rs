//! Export for real hardware (SPEC 3.3 and 4): the project's changes applied to the user's own ROM, written as a
//! patched ROM (for the user's own DS, flashcart or emulator) and/or a BPS patch (to share).
use crate::{
    pack,
    project::{atomic_write, check_path, Project},
    session::RomSession,
    validation::{self, Severity},
    Error, Result,
};
use nitro_formats::{bps, rom_build};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct ExportResult {
    /// ROM files that changed (archive edits roll up to their archive).
    pub changed: Vec<String>,
    /// Files rewritten in place / moved to the end of the ROM.
    pub in_place: Vec<String>,
    pub moved: Vec<String>,
    pub rom_size: u32,
    pub rom_written: Option<String>,
    pub patch_written: Option<String>,
    pub patch_size: Option<u32>,
    /// DS-compatibility and validation notes that didn't stop the export.
    pub warnings: Vec<String>,
}

/// The patched ROM image: every edited file (archive members repacked into their archive) put into the ROM.
pub fn build_rom(
    project: &Project,
    session: &RomSession,
) -> Result<(Vec<u8>, rom_build::BuildReport, Vec<String>)> {
    if project.config.rom_sha256 != session.sha256 {
        return Err(Error::Invalid(
            "the project was made for a different ROM".into(),
        ));
    }
    if session.rom.game_code != "AMCE" || session.rom.revision != 0 {
        return Err(Error::Invalid(
            "Export for DS needs the USA ROM, revision 0 (AMCE)".into(),
        ));
    }
    let mut changes = BTreeMap::new();
    for path in project.edits.keys() {
        check_path(path)?;
        let root = session.root_for(path).ok_or_else(|| {
            Error::Invalid(format!("{path} is a new file; adding files to the ROM isn't supported by Export for DS yet"))
        })?;
        if changes.contains_key(root) {
            continue;
        }
        let original = session
            .rom
            .file(root)
            .ok_or_else(|| Error::Invalid(format!("{root} is not in the ROM")))?;
        changes.insert(
            root.to_string(),
            pack::rebuild_file(root, original, &project.edits)?,
        );
    }
    let changed = changes.keys().cloned().collect();
    let (rom, report) = rom_build::rebuild(session.rom.bytes(), &changes)?;
    Ok((rom, report, changed))
}

/// Builds the patched ROM, checks it, and writes the outputs that were asked for.
pub fn export_ds(
    project: &Project,
    session: &RomSession,
    rom_out: Option<&Path>,
    bps_out: Option<&Path>,
) -> Result<ExportResult> {
    let issues = validation::validate(project, session);
    if let Some(e) = issues
        .iter()
        .find(|i| matches!(i.severity, Severity::Error))
    {
        return Err(Error::Invalid(format!("fix this first: {}", e.message)));
    }
    let mut warnings: Vec<String> = issues.iter().map(|i| i.message.clone()).collect();
    if let Some(base) = &project.config.base {
        warnings.push(format!(
            "Based on {} ({}). Publishing requires the original author's credit or permission.",
            base.source_name, base.source_sha256
        ));
    }
    if rom_out.is_none() && bps_out.is_none() {
        return Err(Error::Invalid("choose a ROM and/or BPS output".into()));
    }
    let (rom, report, mut changed) = build_rom(project, session)?;
    for c in crate::base::changes(session)? {
        if !changed.contains(&c.path) {
            changed.push(c.path);
        }
    }
    let patch = bps_out.map(|_| bps::create(session.clean_bytes(), &rom));
    if let Some(p) = &patch {
        // the patch must reproduce the ROM exactly from the clean one
        if bps::apply(session.clean_bytes(), p)? != rom {
            return Err(Error::Invalid(
                "internal error: the BPS patch does not reproduce the build".into(),
            ));
        }
    }
    for (out, what) in [(rom_out, "ROM"), (bps_out, "patch")] {
        if let Some(path) = out {
            if path
                .canonicalize()
                .is_ok_and(|p| session.path.canonicalize().is_ok_and(|s| p == s))
            {
                return Err(Error::Invalid(format!(
                    "the {what} would overwrite your clean ROM; choose another file"
                )));
            }
        }
    }
    if let (Some(a), Some(b)) = (rom_out, bps_out) {
        let resolve = |p: &Path| -> Result<std::path::PathBuf> {
            Ok(p.parent().unwrap_or(Path::new(".")).canonicalize()?.join(
                p.file_name()
                    .ok_or_else(|| Error::Invalid("missing output name".into()))?,
            ))
        };
        if resolve(a)? == resolve(b)? {
            return Err(Error::Invalid("ROM and patch outputs must differ".into()));
        }
    }
    if let Some(path) = rom_out {
        atomic_write(path, &rom)?;
    }
    if let (Some(path), Some(p)) = (bps_out, &patch) {
        atomic_write(path, p)?;
    }
    Ok(ExportResult {
        changed,
        in_place: report.in_place,
        moved: report.moved,
        rom_size: report.size as u32,
        rom_written: rom_out.map(|p| p.display().to_string()),
        patch_written: bps_out.map(|p| p.display().to_string()),
        patch_size: patch.as_ref().map(|p| p.len() as u32),
        warnings,
    })
}

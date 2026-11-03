use crate::{
    project::{check_edit_conflicts, check_path, ensure_no_symlinks, validate_manifest, Project},
    session::{hash, RomSession},
    Error, Result,
};
use nitro_formats::{archive::Narc, detect};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Write},
    path::PathBuf,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Entry {
    Patch { patch: String },
    Add { add: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageManifest {
    pub format: u32,
    #[serde(flatten, serialize_with = "serialize_package_info")]
    pub info: crate::project::Manifest,
    pub game: String,
    pub files: BTreeMap<String, Entry>,
}
fn serialize_package_info<S: serde::Serializer>(
    info: &crate::project::Manifest,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    let mut value = serde_json::to_value(info).map_err(serde::ser::Error::custom)?;
    // PC is the contract's default. Omitting that optional claim also lets older
    // v1 launchers (which reject unknown fields) install Stage 1's PC packages.
    if info.platforms == ["pc"] {
        if let Some(fields) = value.as_object_mut() {
            fields.remove("platforms");
        }
    }
    value.serialize(serializer)
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct PackResult {
    pub path: String,
    pub size: u32,
    pub warnings: Vec<String>,
}
/// Content-defined chunks resynchronize after insertions; no retail bytes are retained.
fn chunk_hashes(b: &[u8]) -> Vec<(String, usize)> {
    let mut out = vec![];
    let mut start = 0;
    let mut rolling = 0u64;
    for (i, &byte) in b.iter().enumerate() {
        let gear = (byte as u64 + 1)
            .wrapping_mul(0x9e3779b185ebca87)
            .rotate_left(byte as u32 % 63);
        rolling = rolling.wrapping_shl(1).wrapping_add(gear);
        let len = i + 1 - start;
        if len >= 1024 && ((rolling & 2047) == 0 || len >= 8192) {
            out.push((hash(&b[start..=i]), len));
            start = i + 1;
            rolling = 0;
        }
    }
    if start < b.len() {
        out.push((hash(&b[start..]), b.len() - start));
    }
    out
}
fn rebuild(
    path: &str,
    original: &[u8],
    edits: &BTreeMap<String, Vec<u8>>,
    depth: usize,
) -> Result<Vec<u8>> {
    if depth > 8 {
        return Err(Error::Invalid("archive nesting exceeds 8".into()));
    }
    if let Some(bytes) = edits.get(path) {
        return Ok(bytes.clone());
    }
    let prefix = format!("{path}/");
    if !edits.keys().any(|p| p.starts_with(&prefix)) {
        return Ok(original.to_vec());
    }
    let compressed = original.first() == Some(&0x10);
    let plain = if compressed {
        nitro_formats::decompress(original)?
    } else {
        original.to_vec()
    };
    let mut a = Narc::parse(&plain)?;
    let mut recognized = BTreeSet::new();
    for e in &mut a.entries {
        let inner = format!("{prefix}{}", e.name);
        for p in edits.keys() {
            if p == &inner || p.starts_with(&format!("{inner}/")) {
                recognized.insert(p.clone());
            }
        }
        e.data = rebuild(&inner, &e.data, edits, depth + 1)?;
    }
    if edits
        .keys()
        .any(|p| p.starts_with(&prefix) && !recognized.contains(p))
    {
        return Err(Error::Invalid(format!(
            "new archive members are not supported: {path}"
        )));
    }
    let rebuilt = a.write();
    Ok(if compressed {
        nitro_formats::recompress(original, &rebuilt)
    } else {
        rebuilt
    })
}
fn screen_add(
    path: &str,
    b: &[u8],
    session: &RomSession,
    depth: usize,
    budget: &mut usize,
) -> Result<()> {
    if depth > 8 || b.len() > *budget {
        return Err(Error::Invalid(
            "added archives exceed expansion limit".into(),
        ));
    }
    *budget -= b.len();
    if !detect::matches_extension(b, path) {
        return Err(Error::Invalid(format!(
            "new file has unsupported or mismatched format: {path}"
        )));
    }
    if nitro_formats::rom::Rom::parse(b).is_ok() {
        return Err(Error::Invalid(format!(
            "ROM headers are forbidden in add files: {path}"
        )));
    }
    // Compare against every retail leaf, including nested archives. Only hashes are retained here.
    let digest = hash(b);
    for node in &session.nodes {
        let retail = session.file(&node.path)?;
        if hash(retail) == digest {
            return Err(Error::Invalid(format!(
                "add duplicates retail content: {path}"
            )));
        }
        if !b.is_empty() && b.len().abs_diff(retail.len()) <= retail.len() / 20 {
            let blocks = retail.chunks(4096).map(hash).collect::<BTreeSet<_>>();
            let n = b.len().div_ceil(4096);
            let same = b.chunks(4096).filter(|c| blocks.contains(&hash(c))).count();
            if same * 2 >= n {
                return Err(Error::Invalid(format!(
                    "add is near-identical to retail content: {path}"
                )));
            }
        }
        let chunks = chunk_hashes(retail);
        let candidate = chunk_hashes(b);
        if !b.is_empty()
            && candidate
                .iter()
                .filter(|(h, _)| chunks.iter().any(|(rh, _)| rh == h))
                .map(|(_, len)| len)
                .sum::<usize>()
                * 2
                >= b.len()
        {
            return Err(Error::Invalid(format!(
                "add shares at least half its rolling chunks with retail content: {path}"
            )));
        }
    }
    if let Some(f) = detect::detect(b, path) {
        if f.category == "Archive" {
            let plain = if f.compressed {
                nitro_formats::decompress(b)?
            } else {
                b.to_vec()
            };
            for e in Narc::parse(&plain)?.entries {
                let name = if e.name.starts_with('#') {
                    detect::detect(&e.data, &e.name)
                        .map(|f| format!("{}.{}", e.name, f.extension))
                        .unwrap_or(e.name)
                } else {
                    e.name
                };
                screen_add(&name, &e.data, session, depth + 1, budget)?;
            }
        }
    }
    Ok(())
}
pub fn build(project: &Project, session: &RomSession) -> Result<(Vec<u8>, Vec<String>)> {
    validate_manifest(&project.config.manifest)?;
    if project.config.rom_sha256 != session.sha256 {
        return Err(Error::Invalid("target ROM hash changed".into()));
    }
    if session.rom.game_code != "AMCE" || session.rom.revision != 0 {
        return Err(Error::Invalid("packing requires AMCE revision 0".into()));
    }
    let mut roots = BTreeSet::new();
    for path in project.edits.keys() {
        check_path(path)?;
        check_edit_conflicts(project.edits.keys().map(String::as_str), path)?;
        roots.insert(session.root_for(path).unwrap_or(path).to_string());
    }
    let mut case_paths = BTreeSet::new();
    for path in &roots {
        let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if [
            "nds", "srl", "dsi", "sav", "dsv", "gba", "3ds", "cia", "nsp", "xci", "exe", "dll",
            "bat", "cmd", "com", "scr", "msi", "ps1", "vbs", "sh", "jar", "app", "apk",
        ]
        .contains(&extension.as_str())
        {
            return Err(Error::Invalid(format!("blocked package target: {path}")));
        }
        if !case_paths.insert(path.to_lowercase())
            || (session.rom.file(path).is_none()
                && session
                    .rom
                    .files
                    .keys()
                    .any(|p| p.eq_ignore_ascii_case(path)))
        {
            return Err(Error::Invalid(
                "package paths collide when compared without case".into(),
            ));
        }
        let folded = path.to_lowercase();
        if session.rom.files.keys().chain(roots.iter()).any(|other| {
            let other = other.to_lowercase();
            other.starts_with(&format!("{folded}/")) || folded.starts_with(&format!("{other}/"))
        }) {
            return Err(Error::Invalid(
                "a file and a directory share the same package path".into(),
            ));
        }
    }
    let mut manifest = PackageManifest {
        format: 1,
        info: project.config.manifest.clone(),
        game: "AMCE".into(),
        files: BTreeMap::new(),
    };
    let mut payloads = BTreeMap::new();
    let mut warnings = vec![];
    // Unknown compatibility is deliberately conservative in Stage 1.
    if manifest.info.platforms.contains(&"ds".into()) {
        manifest.info.platforms = vec!["pc".into()];
        warnings.push(
            "DS compatibility has not been fully checked in Stage 1; exported as PC only".into(),
        );
    }
    for path in roots {
        if let Some(retail) = session.rom.file(&path) {
            let edited = rebuild(&path, retail, &project.edits, 0)?;
            if edited == retail {
                continue;
            }
            let mut patch = vec![];
            qbsdiff::Bsdiff::new(retail, &edited).compare(&mut patch)?;
            if retail.len() >= 65536 && patch.len() * 100 >= retail.len() * 60 {
                warnings.push(format!(
                    "{path}: patch is at least 60% of the target size; manual review required"
                ));
            }
            let source = format!("patches/{path}.bsdiff");
            check_path(&source)?;
            manifest.files.insert(
                path,
                Entry::Patch {
                    patch: source.clone(),
                },
            );
            payloads.insert(source, patch);
        } else {
            let b = project
                .edits
                .get(&path)
                .ok_or_else(|| Error::Invalid("missing new file".into()))?;
            screen_add(&path, b, session, 0, &mut (64 * 1024 * 1024))?;
            let source = format!("files/{path}");
            check_path(&source)?;
            manifest.files.insert(
                path,
                Entry::Add {
                    add: source.clone(),
                },
            );
            payloads.insert(source, b.clone());
        }
    }
    if payloads.is_empty() {
        return Err(Error::Invalid("there are no changed files to pack".into()));
    }
    if payloads.len() + 1 > 5000 {
        return Err(Error::Invalid("package exceeds 5000 files".into()));
    }
    payloads.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    );
    if payloads.values().map(Vec::len).sum::<usize>() > 400 * 1024 * 1024 {
        return Err(Error::Invalid("package exceeds 400 MiB unpacked".into()));
    }
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for (path, bytes) in payloads {
        zip.start_file(path, opts)?;
        zip.write_all(&bytes)?;
    }
    let bytes = zip.finish()?.into_inner();
    if bytes.len() > 100 * 1024 * 1024 {
        return Err(Error::Invalid("package exceeds 100 MiB".into()));
    }
    Ok((bytes, warnings))
}
pub fn pack(project: &Project, session: &RomSession) -> Result<PackResult> {
    if project.root.as_os_str().is_empty() {
        return Err(Error::Invalid(
            "use build() for an in-memory project".into(),
        ));
    }
    let (bytes, warnings) = build(project, session)?;
    let dir = project.root.join("build");
    ensure_no_symlinks(&project.root, &dir)?;
    std::fs::create_dir_all(&dir)?;
    let path: PathBuf = dir.join(format!(
        "{}-{}.zip",
        project.config.manifest.id, project.config.manifest.version
    ));
    ensure_no_symlinks(&project.root, &path)?;
    crate::project::atomic_write(&path, &bytes)?;
    Ok(PackResult {
        path: path.to_string_lossy().into(),
        size: bytes.len() as u32,
        warnings,
    })
}

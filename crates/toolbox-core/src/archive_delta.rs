//! Format 3: BSDIFF40 on decompressed archive members, never on the LZ stream.
//! Follows the builder's qbsdiff workflow; independent implementation (no source copied).
use crate::{pack::MemberPatch, preview::plain, project::check_path, session::hash, Error, Result};
use nitro_formats::archive::Narc;
use std::collections::{BTreeMap, BTreeSet};

const MAX_EXPANDED: usize = 256 * 1024 * 1024;
const MAX_MEMBER: usize = 64 * 1024 * 1024;
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
fn bounded_plain(bytes: &[u8], budget: &mut usize) -> Result<Vec<u8>> {
    if bytes.len() > MAX_MEMBER {
        return Err(invalid("archive member exceeds 64 MiB"));
    }
    let bytes = plain(bytes)?;
    if bytes.len() > MAX_MEMBER || bytes.len() > *budget {
        return Err(invalid("archive expansion exceeds package limits"));
    }
    *budget -= bytes.len();
    Ok(bytes)
}
pub fn make(
    root: &str,
    before: &[u8],
    after: &[u8],
    payloads: &mut BTreeMap<String, Vec<u8>>,
    warnings: &mut Vec<String>,
) -> Result<BTreeMap<String, MemberPatch>> {
    let mut members = BTreeMap::new();
    let mut budget = MAX_EXPANDED;
    diff(
        root,
        "",
        before,
        after,
        &mut members,
        payloads,
        warnings,
        0,
        &mut budget,
    )?;
    Ok(members)
}
#[allow(clippy::too_many_arguments)]
fn diff(
    root: &str,
    path: &str,
    before: &[u8],
    after: &[u8],
    members: &mut BTreeMap<String, MemberPatch>,
    payloads: &mut BTreeMap<String, Vec<u8>>,
    warnings: &mut Vec<String>,
    depth: usize,
    budget: &mut usize,
) -> Result<()> {
    if before == after {
        return Ok(());
    }
    if depth > 8 {
        return Err(invalid("archive nesting exceeds 8"));
    }
    let source = bounded_plain(before, budget)?;
    let target = bounded_plain(after, budget)?;
    if source.starts_with(b"NARC") && target.starts_with(b"NARC") {
        let a = Narc::parse(&source)?;
        let b = Narc::parse(&target)?;
        if a.entries.len() != b.entries.len()
            || a.entries
                .iter()
                .zip(&b.entries)
                .any(|(a, b)| a.name != b.name)
        {
            return Err(invalid(
                "archive membership/order changes require Export for DS",
            ));
        }
        for (a, b) in a.entries.iter().zip(&b.entries) {
            let child = if path.is_empty() {
                a.name.clone()
            } else {
                format!("{path}/{}", a.name)
            };
            diff(
                root,
                &child,
                &a.data,
                &b.data,
                members,
                payloads,
                warnings,
                depth + 1,
                budget,
            )?;
        }
    } else if source != target {
        check_path(path)?;
        let mut patch = vec![];
        qbsdiff::Bsdiff::new(&source, &target).compare(&mut patch)?;
        if source.len() >= 65536 && patch.len() * 100 >= source.len() * 60 {
            warnings.push(format!("{root}/{path}: member patch is at least 60% of the decompressed target size; manual review required"));
        }
        let payload = format!("patches/{root}/{path}.bsdiff");
        check_path(&payload)?;
        members.insert(
            path.into(),
            MemberPatch {
                patch: payload.clone(),
                source_sha256: hash(&source),
                target_sha256: hash(&target),
            },
        );
        payloads.insert(payload, patch);
    }
    Ok(())
}
/// Reference consumer for launcher integration. Entirely memory-only and atomic on error.
pub fn apply(
    original: &[u8],
    members: &BTreeMap<String, MemberPatch>,
    payloads: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>> {
    if members.is_empty() || members.len() > 4999 {
        return Err(invalid("invalid archive member count"));
    }
    let mut folded = BTreeSet::new();
    for (path, member) in members {
        check_path(path)?;
        check_path(&member.patch)?;
        if !member.patch.ends_with(".bsdiff")
            || !folded.insert(path.to_ascii_lowercase())
            || members.keys().any(|p| {
                p != path
                    && p.to_ascii_lowercase()
                        .starts_with(&format!("{}/", path.to_ascii_lowercase()))
            })
        {
            return Err(invalid("overlapping or ambiguous archive member paths"));
        }
        for digest in [&member.source_sha256, &member.target_sha256] {
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid("invalid member SHA-256"));
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut budget = MAX_EXPANDED;
    let result = apply_inner(original, "", members, payloads, &mut seen, 0, &mut budget)?;
    if seen.len() != members.len() {
        return Err(invalid("archive patch targets a missing member"));
    }
    Ok(result)
}
fn wrap(original: &[u8], rebuilt: &[u8]) -> Vec<u8> {
    if original.first() == Some(&0x10) {
        nitro_formats::recompress(original, rebuilt)
    } else {
        rebuilt.to_vec()
    }
}
fn apply_inner(
    original: &[u8],
    prefix: &str,
    members: &BTreeMap<String, MemberPatch>,
    payloads: &BTreeMap<String, Vec<u8>>,
    seen: &mut BTreeSet<String>,
    depth: usize,
    budget: &mut usize,
) -> Result<Vec<u8>> {
    if depth > 8 {
        return Err(invalid("archive nesting exceeds 8"));
    }
    let source = bounded_plain(original, budget)?;
    let mut archive = Narc::parse(&source)?;
    for entry in &mut archive.entries {
        let path = format!("{prefix}{}", entry.name);
        if let Some(member) = members.get(&path) {
            let source = bounded_plain(&entry.data, budget)?;
            if hash(&source) != member.source_sha256 {
                return Err(invalid(&format!(
                    "{path}: source member hash mismatch (conflicting mod or wrong ROM)"
                )));
            }
            let patch = payloads
                .get(&member.patch)
                .ok_or_else(|| invalid("missing member patch payload"))?;
            if patch.len() > 100 * 1024 * 1024 {
                return Err(invalid("member patch exceeds package limit"));
            }
            let patcher = qbsdiff::Bspatch::new(patch)?;
            let size = patcher.hint_target_size();
            if size > MAX_MEMBER as u64 || size > *budget as u64 {
                return Err(invalid("member patch output exceeds package limit"));
            }
            *budget -= size as usize;
            let mut target = vec![];
            patcher.apply(&source, std::io::Cursor::new(&mut target))?;
            if hash(&target) != member.target_sha256 {
                return Err(invalid("patched member hash mismatch"));
            }
            entry.data = wrap(&entry.data, &target);
            seen.insert(path);
        } else if members.keys().any(|p| p.starts_with(&format!("{path}/"))) {
            entry.data = apply_inner(
                &entry.data,
                &format!("{path}/"),
                members,
                payloads,
                seen,
                depth + 1,
                budget,
            )?;
        }
    }
    Ok(wrap(original, &archive.write()))
}

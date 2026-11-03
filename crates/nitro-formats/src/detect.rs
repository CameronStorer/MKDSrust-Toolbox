// KCL header check ported from CameronStorer/mkds-native-builder (GPL-3.0-or-later).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Format {
    pub extension: &'static str,
    pub category: &'static str,
    pub compressed: bool,
}
pub const FORMATS: &[(&str, &[u8], &str)] = &[
    ("nsbmd", b"BMD0", "Model"),
    ("nsbtx", b"BTX0", "Texture"),
    ("nsbca", b"BCA0", "Animation"),
    ("nsbta", b"BTA0", "Animation"),
    ("nsbtp", b"BTP0", "Animation"),
    ("nsbma", b"BMA0", "Animation"),
    ("nsbva", b"BVA0", "Animation"),
    ("nclr", b"RLCN", "Palette"),
    ("ncgr", b"RGCN", "Tiles"),
    ("nscr", b"RCSN", "Screen"),
    ("ncer", b"RECN", "Cells"),
    ("nanr", b"RNAN", "Animation"),
    ("narc", b"NARC", "Archive"),
    ("sdat", b"SDAT", "Audio"),
    ("sseq", b"SSEQ", "Audio"),
    ("sbnk", b"SBNK", "Audio"),
    ("swar", b"SWAR", "Audio"),
    ("strm", b"STRM", "Audio"),
    ("bmg", b"MESGbmg1", "Text"),
    ("nkm", b"NKMD", "Course"),
];
pub fn detect(b: &[u8], path: &str) -> Option<Format> {
    detect_at_depth(b, path, 0)
}
fn detect_at_depth(b: &[u8], path: &str, depth: usize) -> Option<Format> {
    if b.first() == Some(&0x10) {
        if depth >= 8 {
            return None;
        }
        let plain = crate::decompress(b).ok()?;
        let mut f = detect_at_depth(&plain, path, depth + 1)?;
        f.compressed = true;
        if f.extension == "narc" {
            f.extension = "carc";
        }
        return Some(f);
    }
    if let Some((ext, _, category)) = FORMATS.iter().find(|(_, m, _)| b.starts_with(m)) {
        return Some(Format {
            extension: ext,
            category,
            compressed: false,
        });
    }
    if path
        .rsplit('.')
        .next()
        .is_some_and(|x| x.eq_ignore_ascii_case("kcl"))
        && kcl_header_problem(b).is_none()
        && b.len() >= 0x40
    {
        return Some(Format {
            extension: "kcl",
            category: "Collision",
            compressed: false,
        });
    }
    None
}
pub fn matches_extension(b: &[u8], path: &str) -> bool {
    detect(b, path).is_some_and(|f| {
        path.rsplit('.')
            .next()
            .is_some_and(|x| x.eq_ignore_ascii_case(f.extension))
    })
}

/// KCL has no magic; sanity-check its header offsets exactly as the site does.
fn kcl_header_problem(b: &[u8]) -> Option<String> {
    if b.len() < 0x40 {
        return Some("too short for a KCL header".into());
    }
    let (pos, nrm, tri, oct) = (
        crate::u32le(b, 0).ok()? as usize,
        crate::u32le(b, 4).ok()? as usize,
        crate::u32le(b, 8).ok()? as usize,
        crate::u32le(b, 12).ok()? as usize,
    );
    if !(0x38..=0x40).contains(&pos) {
        return Some("KCL positions offset is not right after the header".into());
    }
    if [pos, nrm, tri, oct].iter().any(|x| x % 4 != 0) {
        return Some("KCL section offsets are not 4-byte aligned".into());
    }
    if !(pos <= nrm && nrm <= tri + 0x10 && tri + 0x10 <= oct && oct < b.len()) {
        return Some("KCL section offsets are out of order or past the end".into());
    }
    if (nrm - pos) % 12 != 0 {
        return Some("KCL position section is not whole vectors".into());
    }
    None
}

//! Structural associations from ASSET_CATALOG.md. No assets or game bytes.
//! NitroPaint links palette/character/screen resources; MKTB keeps course/Tex archives paired.
pub fn normalized(path: &str) -> String {
    let mut p = path.to_ascii_lowercase();
    for lang in ["us", "es", "fr", "ge", "it"] {
        for ext in ["carc", "narc"] {
            p = p.replace(&format!("_{lang}.{ext}"), &format!(".{ext}"));
        }
    }
    p
}
pub fn stem(path: &str) -> String {
    path.rsplit_once('.')
        .map_or(path, |p| p.0)
        .to_ascii_lowercase()
}
/// Ordered, explicit candidates. A missing or ambiguous resource is never substituted at random.
pub fn image_stems(path: &str, format: &str) -> Vec<String> {
    let s = normalized(&stem(path));
    let root = s.strip_suffix(".nce").unwrap_or(&s);
    let base = root
        .strip_suffix("_o")
        .or_else(|| root.strip_suffix("_b"))
        .unwrap_or(root);
    let mut out = vec![s.clone()];
    match format {
        "nclr" => out.extend([format!("{root}_o"), format!("{root}_b")]),
        "ncgr" => {
            out.extend([
                base.into(),
                format!("{base}.nce"),
                format!("{base}_o"),
                format!("{base}_b"),
            ]);
            if let Some(root) = root.strip_suffix("_picture") {
                out.push(format!("{root}_b"));
            }
            if let Some((root, _)) = root.rsplit_once("_bg") {
                out.push(format!("{root}_b"));
            }
            if root.ends_with("/local2") || root.ends_with("/local3") {
                out.push(root[..root.len() - 1].into());
            }
            if root.ends_with("/global1") {
                out.push(root[..root.len() - 1].into());
            }
        }
        "nscr" => {
            out.extend([
                format!("{base}_bg"),
                format!("{base}_bg_00"),
                format!("{base}_picture"),
                base.into(),
            ]);
            if root.ends_with("/local") {
                out.push(format!("{root}2"));
            }
            if root.ends_with("/global") {
                out.push(format!("{root}1"));
            }
        }
        "ncer" => out.extend([base.into(), format!("{base}.nce")]),
        _ => {}
    }
    out.dedup();
    out
}
pub fn model_stem(path: &str) -> String {
    let s = stem(path);
    ["_drive", "_spin", "_win", "_lose"]
        .iter()
        .find_map(|suffix| s.strip_suffix(suffix))
        .unwrap_or(&s)
        .into()
}
pub fn texture_candidates(path: &str) -> Vec<String> {
    let s = model_stem(path);
    let exact = format!("{s}.nsbtx");
    let mut out = vec![exact.clone()];
    if s.contains("/course/") {
        out.push(exact.replace(".carc/", "tex.carc/"));
    }
    if s.ends_with("/killeritem_s") {
        out.push(exact.replace("killeritem_s.", "killeritem."));
    }
    out
}
pub fn model_animation(model: &str, animation: &str) -> bool {
    let m = normalized(&stem(model));
    let a = normalized(&stem(animation));
    a == m
        || ["drive", "spin", "win", "lose"]
            .iter()
            .any(|clip| a == format!("{m}_{clip}"))
}
/// The loose menu directory is mounted by the game; the archive is a stale duplicate.
pub fn used_duplicate(path: &str) -> Option<String> {
    path.strip_prefix("data/KartModelMenu.carc/")
        .map(|p| format!("data/KartModelMenu/{p}"))
}

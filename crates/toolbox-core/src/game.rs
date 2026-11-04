//! Player-facing roles from mkds-data and docs/research; no retail bytes stored here.
use crate::{names::Name, session::FileNode, tables::Catalog};
use serde::Serialize;
use specta::Type;

#[derive(Clone, Debug, Serialize, Type)]
pub struct Entry {
    pub path: String,
    pub group: String,
    pub section: String,
    pub label: String,
    pub location: String,
    pub palette: Option<String>,
    pub screen: Option<String>,
}
pub fn language(path: &str) -> &'static str {
    let p = path.to_ascii_lowercase();
    for (code, label) in [
        ("us", "English (US)"),
        ("en", "English (US)"),
        ("es", "Spanish"),
        ("fr", "French"),
        ("ge", "German"),
        ("it", "Italian"),
        ("ja", "Japanese"),
    ] {
        if p.contains(&format!("_{code}.")) || p.contains(&format!("/{code}.bmg")) {
            return label;
        }
    }
    "Shared / language independent"
}
pub fn text_group(path: &str, index: usize) -> &'static str {
    if path.ends_with("common.bmg") {
        return match index {
            37..=44 => "Cup names",
            45..=76 => "Course names",
            _ => "Common menus",
        };
    }
    if path.contains("kart_select") {
        "Character & kart names"
    } else if path.to_ascii_lowercase().contains("result") {
        "Results & unlocks"
    } else if path.contains("ghost") || path.contains("record") {
        "Records & ghosts"
    } else {
        "Menus & messages"
    }
}
/// Pair exact stems, known screen suffixes and shared/localized archive variants.
/// Ambiguous matches stay in Advanced rather than displaying an unrelated palette.
pub fn companion(files: &[FileNode], path: &str, format: &str) -> Option<String> {
    let stem = path.rsplit_once('.')?.0.to_ascii_lowercase();
    let normalize = |s: &str| {
        let mut s = s.to_ascii_lowercase();
        for code in ["us", "es", "fr", "ge", "it"] {
            s = s.replace(&format!("_{code}.carc"), ".carc");
        }
        s
    };
    let stem = normalize(&stem);
    let mut targets = vec![stem.clone()];
    let root = stem.strip_suffix(".nce").unwrap_or(&stem);
    if format == "nclr" {
        targets.extend([format!("{root}_o"), format!("{root}_b")]);
    }
    if format == "ncgr" {
        targets.push(format!("{root}_o"));
    }
    if format == "nscr" {
        if let Some(base) = stem.strip_suffix("_b") {
            targets.push(format!("{base}_picture"));
        }
        if stem.ends_with("/local") {
            targets.push(format!("{stem}2"));
        }
    }
    for target in targets {
        let matches: Vec<_> = files
            .iter()
            .filter(|f| {
                f.format == format
                    && normalize(f.path.rsplit_once('.').map_or("", |v| v.0)) == target
            })
            .collect();
        if let Some(f) = matches
            .iter()
            .find(|f| language(&f.path) == language(path))
            .or_else(|| {
                matches
                    .iter()
                    .find(|f| language(&f.path).starts_with("Shared"))
            })
            .or_else(|| (matches.len() == 1).then(|| &matches[0]))
        {
            return Some(f.path.clone());
        }
    }
    None
}
pub fn entries(files: &[FileNode], catalog: &Catalog, names: &[Name]) -> Vec<Entry> {
    files
        .iter()
        .filter(|f| {
            !f.archive
                && [
                    "nsbmd", "nsbtx", "nsbca", "nsbta", "nsbtp", "ncgr", "ncer", "bmg", "sdat",
                    "nkm", "kcl",
                ]
                .contains(&f.format.as_str())
        })
        .map(|f| {
            let p = f.path.to_ascii_lowercase();
            let mut group = "Menus & HUD".to_string();
            let mut section = "Screens & icons".to_string();
            let mut label = f
                .name
                .rsplit_once('.')
                .map_or(f.name.as_str(), |v| v.0)
                .replace(".nce", "")
                .replace('_', " ");
            let mut location =
                "On a menu or in the race display; inspect the preview to identify this image"
                    .to_string();
            if let Some((id, _)) = mkds_data::CHARACTERS
                .iter()
                .enumerate()
                .find(|(_, (_, dir))| p.contains(&format!("character/{dir}/")))
            {
                group = "Characters".into();
                section = mkds_data::CHARACTER_NAMES[id].into();
                let role = if p.contains("kartmodelmenu") {
                    "in the character select menu"
                } else if f.name.starts_with("E_") {
                    "far away / as an opponent at a distance"
                } else {
                    "seen when you race as this character"
                };
                location = format!("{section}, {role}");
                let kind = if f.format == "nsbca" {
                    if p.contains("_drive.") {
                        "driving & steering animation"
                    } else if p.contains("_spin.") {
                        "spin-out animation"
                    } else if p.contains("_win.") {
                        "winning animation"
                    } else if p.contains("_lose.") {
                        "losing animation"
                    } else {
                        "animation"
                    }
                } else if f.format == "nsbtx" {
                    "colours & textures"
                } else if f.name.starts_with("E_") {
                    "far-away model"
                } else if p.contains("kartmodelmenu") {
                    "menu model"
                } else {
                    "race model"
                };
                label = format!("{section} {kind}");
            } else if p.contains("/kart/") {
                group = "Karts".into();
                section = "Bodies, wheels & shadows".into();
                location = "On the kart during races or kart selection".into();
                if let Some((id, _)) = mkds_data::CHARACTERS
                    .iter()
                    .enumerate()
                    .find(|(_, (code, _))| f.name.contains(&format!("kart_{code}_")))
                {
                    section = mkds_data::CHARACTER_NAMES[id].into();
                    let variant = if f.name.contains("_a.") {
                        "standard kart"
                    } else if f.name.contains("_b.") {
                        "second kart"
                    } else {
                        "third kart"
                    };
                    label = format!("{section} {variant}");
                }
            } else if p.contains("/course/") {
                group = "Tracks".into();
                section = "Other courses".into();
                if let Some(c) = catalog.courses.iter().find(|c| {
                    p.contains(&format!("/{}.carc/", c.stem.to_ascii_lowercase()))
                        || p.contains(&format!("/{}tex.carc/", c.stem.to_ascii_lowercase()))
                        || (c.variants && p.contains(&format!("/{}d", c.stem.to_ascii_lowercase())))
                }) {
                    for (cup, slots) in catalog.cups.iter().enumerate() {
                        if let Some(pos) = slots.iter().position(|&id| id == c.id) {
                            let name = |i| {
                                names
                                    .iter()
                                    .find(|n| {
                                        language(&n.path) == "English (US)"
                                            && n.path.ends_with("common.bmg")
                                            && n.index == i
                                    })
                                    .map(|n| n.text.clone())
                            };
                            section =
                                name(37 + cup as u32).unwrap_or_else(|| format!("Cup {}", cup + 1));
                            label = format!(
                                "{} · {}",
                                name(45 + cup as u32 * 4 + pos as u32)
                                    .unwrap_or_else(|| c.stem.replace('_', " ")),
                                label
                            );
                            break;
                        }
                    }
                }
                location = if f.format == "kcl" {
                    "The road, walls and surfaces you can drive on"
                } else if f.format == "nkm" {
                    "Race starts, CPU routes, items and checkpoints"
                } else if p.contains("map2d") {
                    "The minimap on the touch screen"
                } else {
                    "The track you see while racing"
                }
                .into();
            } else if f.format == "sdat" {
                group = "Music & sounds".into();
                section = "Songs, voices & sound effects".into();
                label = "Music, character voices & sounds".into();
                location = "Course themes, menu tunes and sounds during play".into();
            } else if f.format == "bmg" {
                group = "Text".into();
                section = language(&f.path).into();
                location = format!("{} · {}", text_group(&f.path, 0), section);
                label = location.clone();
            } else if p.contains("/item/") || p.contains("itembox") {
                group = "Items".into();
                section = "Race items".into();
                location = "Items held by racers or placed on the track".into();
            }
            if group == "Menus & HUD" {
                if p.contains("race_s_o") {
                    label = "Race HUD icons and digits (touch screen)".into();
                    location = "Item roulette, lap and timer display during a race".into();
                } else if p.contains("race_m_o") {
                    label = "Race HUD icons and digits (main screen)".into();
                    location = "The main screen's race display".into();
                } else if p.contains("player_character_l") {
                    label = "Character selection portraits".into();
                    location = "The large character portraits when choosing a driver".into();
                } else if p.contains("select_cup_name") {
                    label = "Cup selection headings".into();
                    location = "The name art above each cup in the selection menu".into();
                } else if let Some(scene) =
                    p.split("/scene/").nth(1).and_then(|s| s.split('.').next())
                {
                    section = scene.replace('_', " ");
                    location = format!("On the {} screen", section);
                }
            }
            if p.contains("/kartmodelmain.carc/") {
                location = "Unused developer reference; the game does not load this archive".into();
            }
            let palette = (f.format == "ncgr" || f.format == "ncer")
                .then(|| companion(files, &f.path, "nclr"))
                .flatten();
            let screen = (f.format == "ncgr")
                .then(|| companion(files, &f.path, "nscr"))
                .flatten();
            Entry {
                path: f.path.clone(),
                group,
                section,
                label,
                location,
                palette,
                screen,
            }
        })
        .collect()
}

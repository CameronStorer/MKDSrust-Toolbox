//! Resolve structural links once on ROM open, checking indexed image compatibility.
use crate::{
    game, preview,
    session::{FileNode, RomSession},
};
use mkds_data::assets;
use nitro_formats::graphics::{Ncgr, Nclr, Nscr};
use serde::Serialize;
use specta::Type;

#[derive(Clone, Debug, Default, Serialize, Type)]
pub struct Pairing {
    pub path: String,
    pub tiles: Option<String>,
    pub palette: Option<String>,
    pub screen: Option<String>,
    pub cells: Option<String>,
    pub textures: Option<String>,
    pub animations: Vec<String>,
    pub face_animation: Option<String>,
}
pub struct ImageIndex<'a> {
    files: std::collections::HashMap<(String, String), Vec<&'a FileNode>>,
}
impl<'a> ImageIndex<'a> {
    pub fn new(files: &'a [FileNode]) -> Self {
        let mut index = Self {
            files: Default::default(),
        };
        for file in files
            .iter()
            .filter(|f| ["ncgr", "nclr", "nscr", "ncer", "nanr"].contains(&f.format.as_str()))
        {
            index
                .files
                .entry((
                    file.format.clone(),
                    assets::normalized(&assets::stem(&file.path)),
                ))
                .or_default()
                .push(file);
        }
        index
    }
    pub fn companion(&self, path: &str, format: &str) -> Option<String> {
        let language = game::language(path);
        for target in assets::image_stems(path, format) {
            let Some(matches) = self.files.get(&(format.into(), target)) else {
                continue;
            };
            let same: Vec<_> = matches
                .iter()
                .filter(|f| game::language(&f.path) == language)
                .collect();
            let shared: Vec<_> = matches
                .iter()
                .filter(|f| game::language(&f.path).starts_with("Shared"))
                .collect();
            let english: Vec<_> = matches
                .iter()
                .filter(|f| game::language(&f.path) == "English (US)")
                .collect();
            let preferred = if !same.is_empty() {
                same
            } else if !shared.is_empty() {
                shared
            } else if language.starts_with("Shared") {
                english
            } else {
                vec![]
            };
            if preferred.len() == 1 {
                return Some(preferred[0].path.clone());
            }
        }
        None
    }
}
pub fn companion(files: &[FileNode], path: &str, format: &str) -> Option<String> {
    ImageIndex::new(files).companion(path, format)
}
pub fn resolve(s: &RomSession) -> Vec<Pairing> {
    let index = ImageIndex::new(&s.nodes);
    let find = |p: &str| {
        s.nodes
            .iter()
            .find(|n| n.path.eq_ignore_ascii_case(p))
            .map(|n| n.path.clone())
    };
    s.nodes
        .iter()
        .filter(|n| {
            ["ncgr", "nscr", "ncer", "nanr", "nclr", "nsbmd", "nsbca"].contains(&n.format.as_str())
        })
        .map(|n| {
            let mut pair = Pairing {
                path: n.path.clone(),
                ..Default::default()
            };
            if n.format.starts_with("nsb") {
                let model_path = if n.format == "nsbmd" {
                    Some(n.path.clone())
                } else {
                    find(&format!("{}.nsbmd", assets::model_stem(&n.path)))
                };
                if let Some(model) = model_path {
                    pair.textures = assets::texture_candidates(&model)
                        .iter()
                        .find_map(|p| find(p));
                    pair.animations = s
                        .nodes
                        .iter()
                        .filter(|a| a.format == "nsbca" && assets::model_animation(&model, &a.path))
                        .map(|a| a.path.clone())
                        .collect();
                    if model
                        .rsplit('/')
                        .next()
                        .is_some_and(|name| name.starts_with("P_"))
                    {
                        pair.face_animation = find(
                            "data/KartModelMainA.carc/character/P_faceanim.nsbtp",
                        )
                        .or_else(|| {
                            s.nodes
                                .iter()
                                .find(|a| a.name == "P_faceanim.nsbtp")
                                .map(|a| a.path.clone())
                        });
                    }
                }
            } else {
                pair.tiles = if n.format == "ncgr" {
                    Some(n.path.clone())
                } else {
                    index.companion(&n.path, "ncgr")
                };
                if let Some(tiles) = pair.tiles.as_deref() {
                    pair.palette = index
                        .companion(tiles, "nclr")
                        .or_else(|| index.companion(&n.path, "nclr"));
                    pair.screen = if n.format == "nscr" {
                        Some(n.path.clone())
                    } else {
                        index.companion(tiles, "nscr")
                    };
                    pair.cells = if n.format == "ncer" {
                        Some(n.path.clone())
                    } else {
                        index.companion(tiles, "ncer")
                    };
                    let read = |path: &str| s.file(path).ok().and_then(|b| preview::plain(b).ok());
                    if let Some(t) = read(tiles).and_then(|b| Ncgr::parse(&b).ok()) {
                        if let Some(p) = pair
                            .palette
                            .as_deref()
                            .and_then(read)
                            .and_then(|b| Nclr::parse(&b).ok())
                        {
                            if t.depth != p.depth
                                || t.pixels.iter().any(|&v| v as usize >= p.colors.len())
                            {
                                pair.palette = None;
                            }
                        }
                        if let Some(map) = pair
                            .screen
                            .as_deref()
                            .and_then(read)
                            .and_then(|b| Nscr::parse(&b).ok())
                        {
                            if map
                                .entries
                                .iter()
                                .any(|&v| (v & 1023) as usize >= t.pixels.len() / 64)
                            {
                                pair.screen = None;
                            }
                        }
                    }
                }
            }
            pair
        })
        .collect()
}

//! Related-asset pages, using the catalog's explicit roles and MKTB's main/Tex course split.
use crate::{names::Name, project::Project, session::RomSession, tables::Catalog};
use serde::Serialize;
use specta::Type;

#[derive(Clone, Debug, Serialize, Type)]
pub struct RelatedAsset {
    pub label: String,
    pub usage: String,
    pub path: Option<String>,
    pub mode: String,
    pub index: u32,
    pub audio_kind: u32,
    pub attachments: Vec<Attachment>,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct Attachment {
    pub path: String,
    pub position: [f32; 3],
    pub scale: f32,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct RelatedPage {
    pub id: String,
    pub kind: String,
    pub slot: u32,
    pub title: String,
    pub assets: Vec<RelatedAsset>,
    pub links: Vec<String>,
    pub stats: Vec<(String, String)>,
}
fn asset(label: &str, usage: &str, path: Option<String>, mode: &str) -> RelatedAsset {
    RelatedAsset {
        label: label.into(),
        usage: usage.into(),
        path,
        mode: mode.into(),
        index: 0,
        audio_kind: 0,
        attachments: vec![],
    }
}
pub fn pages(
    p: Option<&Project>,
    s: &RomSession,
    catalog: &Catalog,
    names: &[Name],
) -> Vec<RelatedPage> {
    let find = |suffix: &str, menu: bool| -> Option<String> {
        s.nodes
            .iter()
            .filter(|n| {
                n.path.ends_with(suffix)
                    && n.path.contains("KartModelMenu") == menu
                    && !n.path.contains("KartModelMain.carc")
            })
            .min_by_key(|n| n.path.contains("KartModelMenu.carc/"))
            .map(|n| n.path.clone())
    };
    let exact = |path: &str| {
        s.nodes
            .iter()
            .find(|n| n.path.eq_ignore_ascii_case(path))
            .map(|n| n.path.clone())
    };
    let read = |path: &str| p.map_or_else(|| s.file(path), |p| p.bytes(s, path));
    let sound = exact("data/Sound/sound_data.sdat");
    let sound_data = sound
        .as_deref()
        .and_then(|path| read(path).ok())
        .and_then(|b| nitro_formats::audio::Sdat::parse(b).ok());
    let mut out = vec![];
    for (slot, (code, dir)) in mkds_data::CHARACTERS.iter().enumerate() {
        let mut assets = vec![
            asset(
                "Race driver",
                "The detailed driver sitting in the kart during a race.",
                find(&format!("character/{dir}/P_{code}.nsbmd"), false),
                "model",
            ),
            asset(
                "Far-away driver",
                "The simpler driver used when detailed rendering is off.",
                find(&format!("character/{dir}/E_{code}.nsbmd"), false),
                "model",
            ),
            asset(
                "Menu driver",
                "The driver on the character selection screen.",
                find(&format!("character/{dir}/P_{code}.nsbmd"), true),
                "model",
            ),
        ];
        for size in ["L", "M"] {
            let mut a = asset(
                &format!("Head icon ({size})"),
                "Shared icon sheet in lists and results; character cell order is inferred.",
                exact(&format!(
                    "data/Main2D_us.carc/player_character_{size}.nce.ncer"
                ))
                .or_else(|| exact(&format!("data/Main2D_us.carc/player_character_{size}.NCER")))
                .or_else(|| exact(&format!("data/Main2D.carc/player_character_{size}.NCER"))),
                "cell",
            );
            a.index = slot as u32;
            assets.push(a);
        }
        let voice = sound_data.as_ref().and_then(|data| {
            let voices =
                nitro_formats::audio::Ssar::parse(&data.files[data.resource_file(1, 2).ok()?])
                    .ok()?;
            if voices.entries.len() != 13 * 14 {
                return None;
            }
            let bank = voices
                .entries
                .get(mkds_data::CHARACTER_VOICE[slot] * 14)?
                .bank as usize;
            data.waves_for_bank(bank)
                .ok()?
                .into_iter()
                .find(|&id| id != 65535)
                .map(u32::from)
        });
        let mut a = asset("Voice", "Voice samples from the bank referenced by this character's voice events; choose a sample and Play.", voice.and(sound.clone()), "audio");
        if let Some(index) = voice {
            a.audio_kind = 3;
            a.index = index;
        }
        assets.push(a);
        out.push(RelatedPage {
            id: format!("character:{slot}"),
            kind: "Character".into(),
            slot: slot as u32,
            title: mkds_data::CHARACTER_NAMES[slot].into(),
            assets,
            links: if slot < 12 {
                (0..3).map(|k| format!("kart:{}", slot * 3 + k)).collect()
            } else {
                vec!["kart:36".into()]
            },
            stats: vec![],
        });
    }
    for slot in 0..37 {
        let Ok((code, dir)) = crate::wizard::kart_id(slot) else {
            continue;
        };
        let mut assets = vec![];
        let offset_path = "data/KartModelMainA.carc/kartoffsetdata.bin";
        let offsets = read(offset_path).ok();
        let at = slot as usize * 224;
        let tire_name = offsets
            .and_then(|b| b.get(at..at + 16))
            .and_then(|b| std::str::from_utf8(b.split(|&v| v == 0).next().unwrap_or_default()).ok())
            .filter(|n| {
                n.starts_with("kart_tire_")
                    && n.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
            })
            .map(String::from)
            .unwrap_or_else(|| format!("kart_tire_{}", ["M", "S", "L"][slot as usize % 3]));
        let scale = offsets
            .and_then(|b| nitro_formats::nkm::fx(b, at + 16).ok())
            .filter(|v| v.is_finite() && *v > 0.)
            .unwrap_or(1.);
        for menu in [false, true] {
            let mut body = asset(
                if menu { "Menu body" } else { "Race body" },
                if menu {
                    "The kart on the kart selection screen."
                } else {
                    "The kart body during a race; wheels use the offset table."
                },
                find(&format!("kart/{dir}/kart_{code}.nsbmd"), menu),
                "model",
            );
            if let Some(tire) = find(&format!("kart/tire/{tire_name}.nsbmd"), menu) {
                if let Some(bytes) = offsets {
                    for i in 0..4 {
                        if let Ok(position) = nitro_formats::nkm::vec3(bytes, at + 20 + i * 12) {
                            body.attachments.push(Attachment {
                                path: tire.clone(),
                                position,
                                scale,
                            });
                        }
                    }
                }
            }
            assets.push(body);
        }
        for size in ["S", "M", "L"] {
            assets.push(asset(
                &format!("Wheels ({size})"),
                "Separate tire models spin and steer at the four wheel positions.",
                find(&format!("kart/tire/kart_tire_{size}.nsbmd"), false),
                "model",
            ));
        }
        assets.push(asset(
            "Shadow",
            "The untextured ground shadow under the kart.",
            find(&format!("kart/shadow/sh_{code}.nsbmd"), false),
            "model",
        ));
        let character = slot as usize / 3;
        let emblem = mkds_data::CHARACTERS
            .get(character)
            .map(|v| v.0)
            .unwrap_or("HH");
        assets.push(asset("Emblem","The sticker applied to the kart at runtime; Standard kart ownership follows the selected character.",find(&format!("emblem/{emblem}_emblem.nsbtx"),false),"image"));
        let stats = catalog
            .karts
            .iter()
            .find(|k| k.id == slot)
            .map(|k| {
                vec![
                    ("Speed".into(), format!("{:.3}", k.speed)),
                    ("Acceleration".into(), format!("{:.3}", k.acceleration)),
                    ("Tires".into(), tire_name),
                    ("Offsets".into(), offset_path.into()),
                ]
            })
            .unwrap_or_default();
        out.push(RelatedPage {
            id: format!("kart:{slot}"),
            kind: "Kart".into(),
            slot,
            title: format!(
                "{} {}",
                mkds_data::CHARACTER_NAMES
                    .get(character)
                    .unwrap_or(&"Shy Guy"),
                ["Standard kart", "kart 2", "kart 3"][slot as usize % 3]
            ),
            assets,
            links: vec![format!("character:{}", character.min(12))],
            stats,
        });
    }
    for c in &catalog.courses {
        let root = format!("data/Course/{}.carc", c.stem);
        if exact(&format!("{root}/course_model.nsbmd")).is_none() {
            continue;
        }
        let title = catalog
            .cups
            .iter()
            .flatten()
            .position(|&id| id == c.id)
            .and_then(|pos| {
                names.iter().find(|n| {
                    n.path.ends_with("common.bmg")
                        && crate::game::language(&n.path) == "English (US)"
                        && n.index == 45 + pos as u32
                })
            })
            .map(|n| n.text.clone())
            .unwrap_or_else(|| c.stem.replace('_', " "));
        let mut music = asset(
            "Music",
            "The course theme selected by the ARM9 music slot table.",
            c.sequence.and(sound.clone()),
            "audio",
        );
        music.index = c.sequence.unwrap_or_default();
        let mut assets = vec![
            asset(
                "Course model",
                "The road and scenery seen during the race.",
                exact(&format!("{root}/course_model.nsbmd")),
                "model",
            ),
            asset(
                "Sky",
                "The sky and distant scenery; some courses have no sky model.",
                exact(&format!("{root}/course_model_V.nsbmd")),
                "model",
            ),
            asset(
                "Collision overlay",
                "Solid surfaces coloured by collision type, over the course model.",
                exact(&format!("{root}/course_collision.kcl")),
                "collision",
            ),
            asset(
                "Minimap",
                "The local minimap on the touch screen.",
                exact(&format!("data/Course/{}Tex.carc/Map2D/local.NCGR", c.stem)),
                "image",
            ),
            asset(
                "Routes and objects",
                "Race starts, checkpoints, CPU routes and placed objects.",
                exact(&format!("{root}/course_map.nkm")),
                "routes",
            ),
            music,
        ];
        for n in s
            .nodes
            .iter()
            .filter(|n| n.format == "nsbmd" && n.path.starts_with(&format!("{root}/MapObj/")))
        {
            assets.push(asset(
                &n.name,
                "An object model used on this course.",
                Some(n.path.clone()),
                "model",
            ));
        }
        out.push(RelatedPage {
            id: format!("track:{}", c.id),
            kind: "Track".into(),
            slot: c.id,
            title,
            assets,
            links: vec![],
            stats: vec![],
        });
    }
    out
}

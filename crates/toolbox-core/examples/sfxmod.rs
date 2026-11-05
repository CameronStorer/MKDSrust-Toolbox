//! Dev helper: list sound-effect wave archives, or replace every sample in matching archives with a test tone
//! through `engine::replace_effect` and install the result as an MKDSrust mod.
//! `sfxmod <rom> list` | `sfxmod <rom> <archive indices or names, comma separated> <project dir> <game dir>`
use std::io::Read;
use toolbox_core::{
    project::{Manifest, Project},
    session::RomSession,
};
const SDAT: &str = "data/Sound/sound_data.sdat";
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let s = RomSession::open(std::path::Path::new(&a[1])).unwrap();
    let sdat = nitro_formats::audio::Sdat::parse(s.file(SDAT).unwrap()).unwrap();
    let archives = sdat.info[3].len();
    if a[2] == "list" {
        for i in 0..archives {
            let Ok(id) = sdat.resource_file(3, i) else {
                continue;
            };
            let Ok(w) = nitro_formats::audio::Swar::parse(&sdat.files[id]) else {
                continue;
            };
            let name = sdat.symbols[3]
                .get(i)
                .cloned()
                .flatten()
                .unwrap_or_default();
            let bytes: usize = w.waves.iter().map(|w| w.data.len()).sum();
            println!("{i:3} {name:32} waves {:4} bytes {bytes}", w.waves.len());
        }
        return;
    }
    let (filter, project_dir, game) = (
        &a[2],
        std::path::Path::new(&a[3]),
        std::path::Path::new(&a[4]),
    );
    // A loud 1 kHz square wave, one second long.
    let tone: Vec<i16> = (0..16000)
        .map(|i| if (i / 8) % 2 == 0 { 20000 } else { -20000 })
        .collect();
    let wav = std::env::temp_dir().join("sfxmod_tone.wav");
    std::fs::write(
        &wav,
        toolbox_core::audio::wav(&toolbox_core::audio::Pcm {
            rate: 16000,
            channels: vec![tone],
        })
        .unwrap(),
    )
    .unwrap();
    let _ = std::fs::remove_dir_all(project_dir);
    let mut p = Project::create(project_dir, &s, Manifest::default()).unwrap();
    let mut replaced = 0;
    for i in 0..archives {
        let name = sdat.symbols[3]
            .get(i)
            .cloned()
            .flatten()
            .unwrap_or_default();
        if !filter
            .split(',')
            .any(|f| f == i.to_string() || (!name.is_empty() && name.contains(f)))
        {
            continue;
        }
        let Ok(id) = sdat.resource_file(3, i) else {
            continue;
        };
        let waves = nitro_formats::audio::Swar::parse(&sdat.files[id])
            .unwrap()
            .waves
            .len();
        for w in 0..waves {
            let options = toolbox_core::engine::EffectOptions {
                archive: i as u32,
                sample: w as u32,
                rate: 16000,
                encoding: 2,
            };
            match toolbox_core::engine::replace_effect(&mut p, &s, SDAT, &wav, options) {
                Ok(()) => replaced += 1,
                Err(e) => println!("{name} wave {w}: {e}"),
            }
        }
    }
    println!("replaced {replaced} samples");
    let (packed, _) = toolbox_core::pack::build_legacy(&p, &s).unwrap();
    let id = p.config.manifest.id.clone();
    let dest = game.join("mods").join(&id);
    let _ = std::fs::remove_dir_all(&dest);
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(packed)).unwrap();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        if f.is_dir() {
            continue;
        }
        let out = dest.join(f.name().unwrap().to_string());
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        let mut b = vec![];
        f.read_to_end(&mut b).unwrap();
        std::fs::write(out, b).unwrap();
    }
    std::fs::write(game.join("mods/enabled.txt"), format!("{id}\n")).unwrap();
    println!("installed");
}

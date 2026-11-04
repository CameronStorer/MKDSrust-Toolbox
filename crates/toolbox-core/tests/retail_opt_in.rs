//! Local opt-in only. Nothing derived from the user's ROM is written to disk.
use nitro_formats::{archive::Narc, bmg::Bmg, graphics::Nclr};
use std::io::{Cursor, Read};
use toolbox_core::{
    pack::{build, Entry, PackageManifest},
    project::{Manifest, Project},
    session::RomSession,
};
#[test]
fn user_rom_roundtrips_and_palette_patch() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("SKIPPED: set MKDS_ROM to test your own AMCE dump");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    assert!(!s.nodes.is_empty());
    let mut palette = None;
    for node in &s.nodes {
        let bytes = s.file(&node.path).unwrap();
        if !["narc", "carc", "bmg", "nclr"].contains(&node.format.as_str()) {
            continue;
        }
        let plain = if node.compressed {
            nitro_formats::decompress(bytes).unwrap()
        } else {
            bytes.to_vec()
        };
        if plain.starts_with(b"NARC") {
            assert_eq!(Narc::parse(&plain).unwrap().write(), plain, "{}", node.path);
        }
        if plain.starts_with(b"MESGbmg1") {
            assert_eq!(Bmg::parse(&plain).unwrap().write(), plain, "{}", node.path);
        }
        if plain.starts_with(b"RLCN") {
            let p = Nclr::parse(&plain).unwrap_or_else(|e| panic!("{}: {e}", node.path));
            assert_eq!(p.write(), plain, "{}", node.path);
            if !p.colors.is_empty() && palette.is_none() {
                palette = Some((node.path.clone(), bytes.to_vec(), p));
            }
        }
    }
    let (path, original, mut colors) = palette.expect("ROM should contain an NCLR palette");
    colors.colors[0] ^= 1;
    let edited = if original.first() == Some(&0x10) {
        nitro_formats::recompress(&original, &colors.write())
    } else {
        colors.write()
    };
    // Project exists solely in memory; even edits/ is never written.
    let project = Project::in_memory(
        &s,
        Manifest::default(),
        std::collections::BTreeMap::from([(path.clone(), edited.clone())]),
    )
    .unwrap();
    let (zip, _) = build(&project, &s).unwrap();
    let mut z = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    let m: PackageManifest = serde_json::from_reader(z.by_name("manifest.json").unwrap()).unwrap();
    let root = s.root_for(&path).unwrap();
    let Entry::Patch { patch } = &m.files[root] else {
        panic!("existing ROM file must be patched")
    };
    let mut patch_bytes = vec![];
    z.by_name(patch)
        .unwrap()
        .read_to_end(&mut patch_bytes)
        .unwrap();
    let mut result = vec![];
    qbsdiff::Bspatch::new(&patch_bytes)
        .unwrap()
        .apply(s.rom.file(root).unwrap(), Cursor::new(&mut result))
        .unwrap();
    let mut current = root.to_string();
    let mut bytes = result;
    while current != path {
        let plain = if bytes.first() == Some(&0x10) {
            nitro_formats::decompress(&bytes).unwrap()
        } else {
            bytes
        };
        let a = Narc::parse(&plain).unwrap();
        let entry = a
            .entries
            .into_iter()
            .find(|e| {
                let full = format!("{current}/{}", e.name);
                path == full || path.starts_with(&format!("{full}/"))
            })
            .expect("edited archive member");
        current = format!("{current}/{}", entry.name);
        bytes = entry.data;
    }
    assert_eq!(bytes, edited);
}

#[test]
fn user_rom_all_requested_formats() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("SKIPPED: set MKDS_ROM for retail format audit");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut failures = vec![];
    for node in &s.nodes {
        let format = node.format.as_str();
        if ![
            "narc", "carc", "bmg", "nclr", "ncgr", "nscr", "nkm", "nsbmd", "nsbtx", "nsbca", "kcl",
        ]
        .contains(&format)
        {
            continue;
        }
        let bytes = toolbox_core::preview::plain(s.file(&node.path).unwrap()).unwrap();
        *counts.entry(format.into()).or_default() += 1;
        let result = (|| -> nitro_formats::Result<()> {
            use nitro_formats::{
                graphics::{Ncgr, Nscr},
                kcl::Kcl,
                model::Nsbmd,
                nkm::Nkm,
                texture::Nsbtx,
            };
            let output = match format {
                "narc" | "carc" => Some(Narc::parse(&bytes)?.write()),
                "bmg" => Some(Bmg::parse(&bytes)?.write()),
                "nclr" => Some(Nclr::parse(&bytes)?.write()),
                "ncgr" => Some(Ncgr::parse(&bytes)?.write()),
                "nscr" => Some(Nscr::parse(&bytes)?.write()),
                "nkm" => Some(Nkm::parse(&bytes)?.write()?),
                "nsbmd" => {
                    Nsbmd::parse(&bytes)?;
                    None
                }
                "nsbtx" => {
                    Nsbtx::parse(&bytes)?;
                    None
                }
                "nsbca" => {
                    nitro_formats::animation::parse(&bytes)?;
                    None
                }
                "kcl" => {
                    Kcl::parse(&bytes)?;
                    None
                }
                _ => unreachable!(),
            };
            if let Some(output) = output {
                assert!(output == bytes, "{}: round trip differs", node.path);
            }
            Ok(())
        })();
        if let Err(e) = result {
            failures.push(format!("{}: {e}", node.path));
        }
    }
    eprintln!("Retail format counts: {counts:?}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn user_rom_collision_rebuild() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("SKIPPED: set MKDS_ROM for KCL rebuild");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    let mut attributes = std::collections::BTreeSet::new();
    let mut rebuilt = false;
    for node in s.nodes.iter().filter(|n| n.format == "kcl") {
        let bytes = toolbox_core::preview::plain(s.file(&node.path).unwrap()).unwrap();
        let k = nitro_formats::kcl::Kcl::parse(&bytes).unwrap();
        attributes.extend(
            k.prisms
                .iter()
                .map(|p| mkds_data::collision_type(p.attribute)),
        );
        if !rebuilt {
            let triangles = k.triangles().unwrap();
            let output = nitro_formats::kcl::Kcl::from_triangles(&triangles)
                .unwrap_or_else(|e| panic!("{}: {e}", node.path));
            assert_eq!(output.prisms.len(), k.prisms.len());
            assert!(!output.octree.is_empty());
            assert_eq!(output.triangles().unwrap().len(), triangles.len());
            eprintln!("Rebuilt retail KCL: {} prisms", triangles.len());
            rebuilt = true;
        }
    }
    eprintln!("Observed retail KCL type values: {attributes:?}");
    assert!(rebuilt);
}

#[test]
fn user_rom_sound_archive_roundtrip() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("SKIPPED: set MKDS_ROM for SDAT audit");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    let mut count = 0;
    for node in s.nodes.iter().filter(|n| n.format == "sdat") {
        let bytes = s.file(&node.path).unwrap();
        let mut sdat = nitro_formats::audio::Sdat::parse(bytes)
            .unwrap_or_else(|e| panic!("{}: {e}", node.path));
        assert!(sdat.write().unwrap() == bytes);
        let mut kinds = std::collections::BTreeMap::<String, usize>::new();
        for (i, b) in sdat.files.iter().enumerate() {
            let kind = String::from_utf8_lossy(&b[..4]).to_string();
            *kinds.entry(kind.clone()).or_default() += 1;
            let result = (|| -> nitro_formats::Result<()> {
                use nitro_formats::audio::*;
                let rebuilt = match kind.as_str() {
                    "SSEQ" => Sseq::parse(b)?.write(),
                    "SBNK" => Sbnk::parse(b)?.write()?,
                    "SWAR" => Swar::parse(b)?.write()?,
                    "SSAR" => Ssar::parse(b)?.write()?,
                    "STRM" => Strm::parse(b)?.write()?,
                    _ => panic!("unknown audio child kind"),
                };
                assert!(rebuilt == *b, "audio child {i} round trip");
                Ok(())
            })();
            result.unwrap_or_else(|e| panic!("audio child {i} ({kind}): {e}"));
        }
        eprintln!("Retail SDAT child counts: {kinds:?}");
        let sequence = sdat.info[0]
            .iter()
            .position(Option::is_some)
            .expect("retail sequence");
        let preview = toolbox_core::audio::preview_sequence(&sdat, sequence, 0.1).unwrap();
        assert!(!preview.channels[0].is_empty());
        // Force relocation through the same writer used for an imported song.
        let first = sdat
            .files
            .iter()
            .position(|b| b.starts_with(b"SSEQ"))
            .unwrap();
        let original = sdat.files[first].clone();
        let mut seq = nitro_formats::audio::Sseq::parse(&original).unwrap();
        seq.events.push(0xff);
        sdat.files[first] = seq.write();
        let rebuilt = nitro_formats::audio::Sdat::parse(&sdat.write().unwrap()).unwrap();
        assert_eq!(rebuilt.files.len(), sdat.files.len());
        assert!(rebuilt.files == sdat.files);
        count += 1;
    }
    assert!(count > 0);
}
#[test]
fn user_rom_3d_formats_and_course_export() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("SKIPPED: set MKDS_ROM for local 3D format coverage");
        return;
    };
    let s = RomSession::open(std::path::Path::new(&path)).unwrap();
    let mut course = None;
    for node in &s.nodes {
        if !["nsbmd", "kcl", "nkm"].contains(&node.format.as_str()) {
            continue;
        }
        let bytes = toolbox_core::preview::plain(s.file(&node.path).unwrap()).unwrap();
        match node.format.as_str() {
            "nsbmd" => {
                nitro_formats::model::Nsbmd::parse(&bytes)
                    .unwrap_or_else(|e| panic!("{}: {e}", node.path));
            }
            "kcl" => {
                nitro_formats::kcl::Kcl::parse(&bytes)
                    .unwrap_or_else(|e| panic!("{}: {e}", node.path));
            }
            "nkm" => {
                assert_eq!(
                    nitro_formats::nkm::Nkm::parse(&bytes)
                        .unwrap_or_else(|e| panic!("{}: {e}", node.path))
                        .write()
                        .unwrap(),
                    bytes,
                    "{}",
                    node.path
                );
                course.get_or_insert(node.path.clone());
            }
            _ => unreachable!(),
        }
    }
    let scene =
        toolbox_core::scene::CourseScene::load(None, &s, &course.expect("ROM contains a course"))
            .unwrap();
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("course.glb"), scene.glb(false).unwrap()).unwrap();
    std::fs::write(
        temp.path().join("course.obj"),
        toolbox_core::model::obj(scene.kcl.as_ref().expect("course KCL")).unwrap(),
    )
    .unwrap();
}

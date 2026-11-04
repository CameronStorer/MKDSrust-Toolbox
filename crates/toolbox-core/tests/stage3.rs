use nitro_formats::{
    bps,
    graphics::{Image, Ncgr, Nclr},
    texture::{self, Nsbtx},
};
use std::{collections::BTreeMap, path::PathBuf};
use toolbox_core::{
    base, export, graphic, pack, preview,
    project::{Manifest, Project},
    service::{Request, Service},
    session::RomSession,
    synthetic,
};
fn session() -> RomSession {
    RomSession::from_bytes(synthetic::rom(), PathBuf::new()).unwrap()
}
#[test]
fn tile_paint_png_conversion_history_and_ds_export() {
    let temp = tempfile::tempdir().unwrap();
    let s = session();
    let mut p = Project::create(&temp.path().join("project"), &s, Manifest::default()).unwrap();
    let tile = "data/Studio/demo.carc/art/studio.ncgr";
    let pal = "data/Studio/demo.carc/art/studio.nclr";
    let before = p.bytes(&s, tile).unwrap().to_vec();
    graphic::paint(&mut p, &s, tile, Some(pal), 0, 0, &[(0, 0, 7), (8, 0, 8)]).unwrap();
    let back = Ncgr::parse(p.bytes(&s, tile).unwrap()).unwrap();
    assert_eq!((back.pixels[0], back.pixels[64]), (7, 8));
    p.undo().unwrap();
    assert_eq!(p.bytes(&s, tile).unwrap(), before);
    p.redo().unwrap();
    let image = Image {
        width: 128,
        height: 128,
        rgba: (0..128 * 128)
            .flat_map(|i| [(i % 256) as u8, (i / 128) as u8, 120, 255])
            .collect(),
    };
    let source = temp.path().join("own.png");
    std::fs::write(&source, preview::png(&image).unwrap()).unwrap();
    let draft = graphic::import(&p, &s, tile, &source, Some(pal), 0, 0, 4, true).unwrap();
    assert_eq!(draft.info.colors, 256);
    let old_pal = p.bytes(&s, pal).unwrap().to_vec();
    p.edit_many(draft.edits, "PNG", &s).unwrap();
    assert_ne!(p.bytes(&s, pal).unwrap(), old_pal);
    p.undo().unwrap();
    assert_eq!(p.bytes(&s, pal).unwrap(), old_pal);
    p.redo().unwrap();
    let output = temp.path().join("out.nds");
    let patch = temp.path().join("out.bps");
    let r = export::export_ds(&p, &s, Some(&output), Some(&patch)).unwrap();
    assert_eq!(r.changed, ["data/Studio/demo.carc"]);
    let rebuilt = std::fs::read(output).unwrap();
    assert_eq!(
        bps::apply(s.clean_bytes(), &std::fs::read(patch).unwrap()).unwrap(),
        rebuilt
    );
    assert_eq!(
        RomSession::from_bytes(rebuilt, PathBuf::new())
            .unwrap()
            .file(tile)
            .unwrap(),
        p.bytes(&s, tile).unwrap()
    );
}
#[test]
fn all_texture_encoders_replace_parse_and_paint_without_growth() {
    let temp = tempfile::tempdir().unwrap();
    let s = session();
    let mut p = Project::create(&temp.path().join("project"), &s, Manifest::default()).unwrap();
    let path = "data/Studio/texture.nsbtx";
    let image = Image {
        width: 8,
        height: 8,
        rgba: (0..64)
            .flat_map(|i| [(i * 4) as u8, 40, 200, if i % 4 == 0 { 0 } else { 255 }])
            .collect(),
    };
    for format in [1, 2, 3, 4, 6, 7] {
        let count = match format {
            1 => 32,
            2 => 4,
            3 => 16,
            4 => 256,
            6 => 8,
            _ => 2,
        };
        let (palette, indices) =
            graphic::quantize(&image, count, false, [2, 3, 4].contains(&format));
        let t = texture::encode(
            &image,
            format,
            &palette,
            &indices,
            [2, 3, 4].contains(&format),
        )
        .unwrap();
        let tx = Nsbtx::parse(s.file(path).unwrap()).unwrap();
        let bytes = tx
            .replace(0, 0, &t, (format != 7).then_some(palette.as_slice()))
            .unwrap();
        let parsed = Nsbtx::parse(&bytes).unwrap();
        assert_eq!(parsed.textures[0].format, format);
        assert_eq!(parsed.decode(0, 0).unwrap().width, 8);
        p.edit(path, bytes, "convert", &s).unwrap();
        let len = p.bytes(&s, path).unwrap().len();
        for color in [0, 1, 2] {
            graphic::paint(&mut p, &s, path, None, 0, 0, &[(0, 0, color)]).unwrap();
            assert_eq!(p.bytes(&s, path).unwrap().len(), len);
        }
    }
    // Repeated format changes repack live pools instead of retaining dead images.
    let mut tx = Nsbtx::parse(s.file(path).unwrap()).unwrap();
    for _ in 0..4 {
        for format in [4, 7] {
            let (pal, indices) = graphic::quantize(&image, 256, false, true);
            let value = texture::encode(&image, format, &pal, &indices, true).unwrap();
            let bytes = tx
                .replace(0, 0, &value, (format != 7).then_some(pal.as_slice()))
                .unwrap();
            assert!(bytes.len() < 2048);
            tx = Nsbtx::parse(&bytes).unwrap();
        }
    }
    let translucent = Image {
        width: 8,
        height: 8,
        rgba: [240, 16, 96, 64].repeat(64),
    };
    let (colors, indices) = graphic::quantize(&translucent, 32, false, false);
    let encoded = texture::encode(&translucent, 1, &colors, &indices, false).unwrap();
    let decoded = texture::decode(&encoded, &colors).unwrap();
    assert!(decoded.rgba[0] > 200);
    assert!(decoded.rgba[3] > 0 && decoded.rgba[3] < 128);
    let mut direct_only = s.file(path).unwrap().to_vec();
    let tex_at = u32::from_le_bytes(direct_only[16..20].try_into().unwrap()) as usize;
    let pal_at = u32::from_le_bytes(direct_only[tex_at + 52..tex_at + 56].try_into().unwrap())
        as usize
        + tex_at;
    direct_only[pal_at + 1] = 0;
    let tx = Nsbtx::parse(&direct_only).unwrap();
    assert!(tx.palettes.is_empty());
    let (colors, indices) = graphic::quantize(&image, 16, false, true);
    let value = texture::encode(&image, 3, &colors, &indices, true).unwrap();
    let converted = tx.replace(0, 0, &value, Some(&colors)).unwrap();
    let converted = Nsbtx::parse(&converted).unwrap();
    assert_eq!(converted.palettes[0].name, "studio");
    assert_eq!(converted.decode(0, 0).unwrap().width, 8);
}
fn base_loop(s: RomSession) {
    let temp = tempfile::tempdir().unwrap();
    let pals: Vec<_> = s
        .nodes
        .iter()
        .filter(|n| n.format == "nclr")
        .take(2)
        .map(|n| n.path.clone())
        .collect();
    // The synthetic ROM has one palette; use a different text file for the second edit.
    let a = pals.first().unwrap();
    let b = s
        .nodes
        .iter()
        .find(|n| n.format == "bmg")
        .unwrap()
        .path
        .clone();
    let mut palette = Nclr::parse(&preview::plain(s.file(a).unwrap()).unwrap()).unwrap();
    palette.colors[0] ^= 1;
    let original = s.file(a).unwrap();
    let a_bytes = if original.first() == Some(&0x10) {
        nitro_formats::recompress(original, &palette.write())
    } else {
        palette.write()
    };
    let root = s.root_for(a).unwrap();
    let replacement = pack::rebuild_file(
        root,
        s.rom.file(root).unwrap(),
        &BTreeMap::from([(a.clone(), a_bytes.clone())]),
    )
    .unwrap();
    let (hack, _, _) = export::build_rom(
        &Project::in_memory(
            &s,
            Manifest::default(),
            BTreeMap::from([(root.into(), replacement)]),
        )
        .unwrap(),
        &s,
    )
    .unwrap();
    let source = temp.path().join("someone.bps");
    std::fs::write(&source, bps::create(s.clean_bytes(), &hack)).unwrap();
    let (mut p, working) = base::create(
        &temp.path().join("project"),
        &s,
        Manifest::default(),
        &source,
    )
    .unwrap();
    assert_eq!(working.file(a).unwrap(), a_bytes);
    let mut text =
        nitro_formats::bmg::Bmg::parse(&preview::plain(working.file(&b).unwrap()).unwrap())
            .unwrap();
    text.messages[0]
        .tokens
        .insert(0, nitro_formats::bmg::Token::Text(vec![65]));
    let original = working.file(&b).unwrap();
    let b_bytes = if original.first() == Some(&0x10) {
        nitro_formats::recompress(original, &text.write())
    } else {
        text.write()
    };
    p.edit(&b, b_bytes.clone(), "another edit", &working)
        .unwrap();
    let reopened = base::working(&p.root, &s).unwrap();
    let p = Project::open(&p.root, &reopened).unwrap();
    let final_patch = temp.path().join("final.bps");
    export::export_ds(&p, &reopened, None, Some(&final_patch)).unwrap();
    let final_bytes = bps::apply(s.clean_bytes(), &std::fs::read(final_patch).unwrap()).unwrap();
    let final_session = RomSession::from_bytes(final_bytes, PathBuf::new()).unwrap();
    assert_eq!(final_session.file(a).unwrap(), a_bytes);
    assert_eq!(final_session.file(&b).unwrap(), b_bytes);
    assert!(p.root.join("base.bps").is_file());
    assert!(!p.root.join("someone.nds").exists());
    assert_eq!(p.config.base.as_ref().unwrap().source_name, "someone.bps");
}
#[test]
fn imported_base_plus_edits_exports_against_clean() {
    base_loop(session());
}
#[test]
fn user_rom_base_plus_edit() {
    let Some(path) = std::env::var_os("MKDS_ROM") else {
        eprintln!("SKIPPED: MKDS_ROM not set");
        return;
    };
    base_loop(RomSession::open(std::path::Path::new(&path)).unwrap());
}
#[test]
fn service_export_protects_clean_and_distinct_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let clean = temp.path().join("clean.nds");
    std::fs::write(&clean, synthetic::rom()).unwrap();
    let s = RomSession::open(&clean).unwrap();
    let p = Project::create(&temp.path().join("project"), &s, Manifest::default()).unwrap();
    let mut service = Service {
        session: Some(s),
        project: Some(p),
        persist_settings: false,
        ..Service::default()
    };
    assert!(service
        .dispatch(Request::ExportDs {
            rom: Some(clean.display().to_string()),
            bps: None
        })
        .is_err());
    let out = temp.path().join("out.bps").display().to_string();
    assert!(service
        .dispatch(Request::ExportDs {
            rom: Some(out.clone()),
            bps: Some(out)
        })
        .is_err());
    assert_eq!(std::fs::read(clean).unwrap(), synthetic::rom());
}

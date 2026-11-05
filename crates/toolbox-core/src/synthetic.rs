//! Entirely generated demonstration data. No retail data is used here.
use nitro_formats::{
    archive::Narc,
    bmg::{Bmg, Token},
    graphics::{Ncgr, Nclr, Nscr},
};
use std::collections::BTreeMap;
fn put(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
pub fn files() -> BTreeMap<String, Vec<u8>> {
    let authored_model = nitro_formats::model::Nsbmd::parse(&nitro_formats::synthetic::nsbmd())
        .unwrap()
        .write()
        .unwrap();
    let colors = (0..256)
        .map(|i| ((i % 32) | ((i / 8 % 32) << 5) | ((31 - i / 8 % 32) << 10)) as u16)
        .collect::<Vec<_>>();
    let palette = Nclr::new(&colors, true, false).write();
    let pixels = (0..256 * 64)
        .map(|i| {
            let tile = i / 64;
            let x = i % 8;
            let y = i / 8 % 8;
            ((tile * 3 + x * 2 + y * 5) % 255 + 1) as u8
        })
        .collect();
    let tiles = Ncgr::new(16, 16, 4, false, pixels)
        .expect("generated tiles")
        .write();
    let screen = Nscr::new(128, 128, &(0..256).map(|i| i as u16).collect::<Vec<_>>())
        .expect("generated map")
        .write();
    let mut text = Bmg::new(&[
        "Welcome to your next idea.",
        "Take the scenic route.",
        "Ready when you are.",
        "A little colour goes a long way.",
        "Made by you.",
    ]);
    text.messages[1].tokens = vec![
        Token::Text("Take the ".encode_utf16().collect()),
        Token::Code(vec![0x1a, 0, 6, 1, 0, 0]),
        Token::Text("scenic route.".encode_utf16().collect()),
    ];
    let inner = Narc::new(BTreeMap::from([
        ("audio/sound_data.sdat".into(), sound_fixture()),
        ("art/studio.nclr".into(), palette),
        ("art/studio.ncgr".into(), tiles),
        ("art/studio.nscr".into(), screen),
        ("art/studio.ncer".into(), nitro_formats::synthetic::ncer()),
        ("art/studio.nanr".into(), nitro_formats::synthetic::nanr()),
        ("language/en.bmg".into(), text.write()),
    ]))
    .expect("generated archive")
    .write();
    BTreeMap::from([
        (
            "data/Course/studio.carc".into(),
            nitro_formats::compress(
                &Narc::new(BTreeMap::from([
                    ("course_model.nsbmd".into(), authored_model),
                    ("course_model.nsbtx".into(), texture_fixture()),
                    (
                        "course_model.nsbca".into(),
                        nitro_formats::synthetic::nsbca(),
                    ),
                    (
                        "course_collision.kcl".into(),
                        nitro_formats::synthetic::kcl(),
                    ),
                    ("course_map.nkm".into(), nitro_formats::synthetic::nkm()),
                ]))
                .expect("synthetic course archive")
                .write(),
            ),
        ),
        (
            "data/Studio/demo.carc".into(),
            nitro_formats::compress(&inner),
        ),
        (
            "data/Studio/readout.bmg".into(),
            Bmg::new(&["Synthetic workspace", "Nothing here comes from a game."]).write(),
        ),
        ("data/Studio/texture.nsbtx".into(), texture_fixture()),
    ])
}
pub fn sound_fixture() -> Vec<u8> {
    use nitro_formats::audio::Sdat;
    let pcm = crate::audio::Pcm {
        rate: 16000,
        channels: vec![(0..8000)
            .map(|i| ((i as f32 / 16000. * 440. * std::f32::consts::TAU).sin() * 10000.) as i16)
            .collect()],
    };
    let song = crate::audio::sampled(&pcm, 16000, false, 1, Some(0), None).expect("synthetic tone");
    let mut s = Sdat::default();
    s.files = vec![
        song.sequence.write(),
        song.bank.write().unwrap(),
        song.waves.write().unwrap(),
    ];
    s.info[0].push(Some(vec![0, 0, 0, 0, 0, 0, 127, 64, 64, 0]));
    s.info[2].push(Some(vec![1, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255]));
    s.info[3].push(Some(vec![2, 0, 0, 0]));
    s.info[4].push(Some(vec![1, 0, 255, 255, 0, 0, 1, 0]));
    s.symbols[0].push(Some("Original synth tone".into()));
    s.symbols[2].push(Some("Sine bank".into()));
    s.symbols[3].push(Some("Sine waves".into()));
    s.write().unwrap()
}
/// An 8×8 direct-colour texture in a real TEX0 dictionary.
pub fn texture_fixture() -> Vec<u8> {
    fn dict(stride: usize, name: &str, data: &[u8]) -> Vec<u8> {
        let len = 16 + 4 + stride + 16;
        let mut b = vec![0; len];
        b[1] = 1;
        b[2..4].copy_from_slice(&(len as u16).to_le_bytes());
        b[4..6].copy_from_slice(&8u16.to_le_bytes());
        b[6..8].copy_from_slice(&8u16.to_le_bytes());
        b[8..12].copy_from_slice(&0x17fu32.to_le_bytes());
        b[16..18].copy_from_slice(&(stride as u16).to_le_bytes());
        b[18..20].copy_from_slice(&((4 + stride) as u16).to_le_bytes());
        b[20..20 + stride].copy_from_slice(data);
        b[20 + stride..20 + stride + name.len()].copy_from_slice(name.as_bytes());
        b
    }
    let texture = dict(8, "studio", &((7u64 << 26).to_le_bytes()));
    let pal = dict(4, "studio_pl", &[0; 4]);
    let mut t = vec![0; 60];
    t[..4].copy_from_slice(b"TEX0");
    t[12..14].copy_from_slice(&16u16.to_le_bytes());
    t[14..16].copy_from_slice(&60u16.to_le_bytes());
    let pd = 60 + texture.len();
    put(&mut t, 52, pd as u32);
    let base = pd + pal.len();
    put(&mut t, 20, base as u32);
    put(&mut t, 36, base as u32);
    put(&mut t, 40, base as u32);
    put(&mut t, 56, (base + 128) as u32);
    t[48..50].copy_from_slice(&1u16.to_le_bytes());
    t.extend_from_slice(&texture);
    t.extend_from_slice(&pal);
    for i in 0..64u16 {
        let c = 0x8000 | (i % 8 * 4) | ((i / 8 * 4) << 5) | (18 << 10);
        t.extend_from_slice(&c.to_le_bytes());
    }
    t.extend_from_slice(&[0; 8]);
    let len = t.len() as u32;
    put(&mut t, 4, len);
    let mut b = b"BTX0\xff\xfe\x01\0\0\0\0\0\x10\0\x01\0\x14\0\0\0".to_vec();
    b.extend_from_slice(&t);
    let len = b.len() as u32;
    put(&mut b, 8, len);
    b
}
pub fn rom() -> Vec<u8> {
    rom_with(files())
}
/// Complete structural slot fixture for beginner-recipe integration tests.
/// Every model, clip, texture, name and sound here is generated by this repository.
pub fn wizard_rom() -> Vec<u8> {
    let mut files = files();
    let model = nitro_formats::synthetic::nsbmd();
    let tx = texture_fixture();
    let portrait_tiles = Ncgr::new(
        16,
        16,
        4,
        false,
        (0..128 * 128).map(|i| (i % 255 + 1) as u8).collect(),
    )
    .unwrap()
    .write();
    let portrait_palette = Nclr::new(
        &(0..256).map(|i| i as u16 * 100 % 32768).collect::<Vec<_>>(),
        true,
        false,
    )
    .write();
    for (archive, members) in [
        (
            "Scene/Race_us",
            BTreeMap::from([("race_s_o.NCGR".into(), portrait_tiles.clone())]),
        ),
        (
            "Scene/Race",
            BTreeMap::from([
                ("race_s_o.NCLR".into(), portrait_palette.clone()),
                ("race_s.NCER".into(), nitro_formats::synthetic::ncer()),
            ]),
        ),
    ] {
        files.insert(
            format!("data/{archive}.carc"),
            Narc::new(members).unwrap().write(),
        );
    }
    let mut item = nitro_formats::model::Nsbmd::parse(&model).unwrap();
    item.textures = Some(nitro_formats::texture::Nsbtx::parse(&tx).unwrap());
    files.insert(
        "data/MainRace.carc".into(),
        Narc::new(BTreeMap::from([(
            "Item/it_studio.nsbmd".into(),
            item.write().unwrap(),
        )]))
        .unwrap()
        .write(),
    );
    let mut archives: BTreeMap<String, BTreeMap<String, Vec<u8>>> = BTreeMap::new();
    for (id, (code, dir)) in mkds_data::CHARACTERS.iter().enumerate() {
        for archive in [
            if id == 12 {
                "KartModelMainB"
            } else {
                "KartModelSub"
            },
            "KartModelMenu",
        ] {
            let a = archives.entry(archive.into()).or_default();
            a.insert(format!("character/{dir}/P_{code}.nsbmd"), model.clone());
            a.insert(format!("character/{dir}/P_{code}.nsbtx"), tx.clone());
            for clip in ["drive", "spin", "win", "lose"] {
                a.insert(
                    format!("character/{dir}/P_{code}_{clip}.nsbca"),
                    nitro_formats::synthetic::nsbca(),
                );
            }
        }
        let a = archives.entry("KartModelMainA".into()).or_default();
        a.insert(format!("character/{dir}/E_{code}.nsbmd"), model.clone());
        a.insert(format!("character/{dir}/E_{code}.nsbtx"), tx.clone());
    }
    for id in 0..36 {
        let (code, dir) = crate::wizard::kart_id(id).unwrap();
        for archive in ["KartModelMainA", "KartModelMenu"] {
            let a = archives.entry(archive.into()).or_default();
            a.insert(format!("kart/{dir}/kart_{code}.nsbmd"), model.clone());
            a.insert(format!("kart/{dir}/kart_{code}.nsbtx"), tx.clone());
            a.insert(format!("kart/shadow/sh_{code}.nsbmd"), model.clone());
        }
    }
    for archive in ["KartModelMainA", "KartModelMenu"] {
        let a = archives.entry(archive.into()).or_default();
        for (file, stride) in [
            ("kartoffsetdata.bin", 224),
            (
                "kartphysicalparam.bin",
                if archive == "KartModelMainA" {
                    152
                } else {
                    148
                },
            ),
            ("kartmenuparam.bin", 12),
            ("kartAIparam.bin", 160),
        ] {
            a.insert(file.into(), vec![0; stride * 50]);
        }
    }
    for (name, entries) in archives {
        files.insert(
            format!("data/{name}.carc"),
            nitro_formats::compress(&Narc::new(entries).unwrap().write()),
        );
    }
    let names: Vec<_> = (0..140).map(|i| format!("Studio name {i}")).collect();
    let spanish: Vec<_> = (0..140).map(|i| format!("Prueba {i}")).collect();
    files.insert(
        "data/Main2D_es.carc".into(),
        Narc::new(BTreeMap::from([(
            "common.bmg".into(),
            Bmg::new(&spanish.iter().map(String::as_str).collect::<Vec<_>>()).write(),
        )]))
        .unwrap()
        .write(),
    );
    let refs: Vec<_> = names.iter().map(String::as_str).collect();
    files.insert(
        "data/Main2D_us.carc".into(),
        Narc::new(BTreeMap::from([
            ("common.bmg".into(), Bmg::new(&refs).write()),
            ("player_character_L.nce.ncgr".into(), portrait_tiles),
            ("player_character_L_o.NCLR".into(), portrait_palette),
            (
                "player_character_L.nce.ncer".into(),
                nitro_formats::synthetic::ncer(),
            ),
        ]))
        .unwrap()
        .write(),
    );
    let mut sound = nitro_formats::audio::Sdat::parse(&sound_fixture()).unwrap();
    sound.info[2].resize(16, sound.info[2][0].clone());
    let mut events = vec![];
    let mut entries = vec![];
    for character in 0..13 {
        let offset = events.len() as u32;
        events.extend([60 + character, 100, 1, 255]);
        for _ in 0..14 {
            entries.push(nitro_formats::audio::SequenceArchiveEntry {
                offset: Some(offset),
                bank: 3 + character as u16,
                volume: 100,
                priorities: [64, 64, 0],
            });
        }
    }
    let id = sound.files.len() as u32;
    sound.files.push(
        nitro_formats::audio::Ssar::new(entries, events)
            .unwrap()
            .write()
            .unwrap(),
    );
    sound.info[1].resize(3, None);
    sound.info[1][2] = Some(id.to_le_bytes().to_vec());
    files.insert("data/Sound/sound_data.sdat".into(), sound.write().unwrap());
    // Authored reference course for synthetic budget checks. It has realistic
    // capacities without embedding a single byte from a retail course.
    let mut reference = nitro_formats::model::Nsbmd::parse(&model).unwrap();
    let mut mesh = nitro_formats::model::Mesh {
        name: "reference".into(),
        ..Default::default()
    };
    for z in 0..32 {
        for x in 0..32 {
            let i = mesh.positions.len() as u32;
            let x = x as f32 * 60.;
            let z = z as f32 * 60.;
            mesh.positions.extend([
                [x, 0., z],
                [x + 60., 0., z],
                [x, 0., z + 60.],
                [x + 60., 0., z + 60.],
            ]);
            mesh.indices.extend([i, i + 2, i + 1, i + 1, i + 2, i + 3]);
        }
    }
    reference.models[0].meshes = vec![mesh];
    let collision = crate::collision::generate(
        &reference.models[0],
        &crate::collision::AttributeTable::default(),
    )
    .unwrap();
    let route = crate::track_wizard::generate_nkm(
        20,
        3,
        &[
            [0., 0., 0.],
            [1800., 0., 0.],
            [1800., 0., 1800.],
            [0., 0., 1800.],
        ],
        150.,
        32,
    )
    .unwrap();
    files.insert(
        "data/Course/reference.carc".into(),
        Narc::new(BTreeMap::from([
            ("course_model.nsbmd".into(), reference.write().unwrap()),
            ("course_collision.kcl".into(), collision.write()),
            ("course_map.nkm".into(), route.write().unwrap()),
        ]))
        .unwrap()
        .write(),
    );
    files.insert(
        "data/Course/referenceTex.carc".into(),
        Narc::new(BTreeMap::from([
            (
                "Map2D/local.NCGR".into(),
                Ncgr::new(32, 32, 4, false, vec![0; 65536]).unwrap().write(),
            ),
            (
                "Map2D/local.NCLR".into(),
                Nclr::new(&vec![0; 256], true, false).write(),
            ),
            (
                "Map2D/local2.NSCR".into(),
                Nscr::new(256, 256, &vec![0; 1024]).unwrap().write(),
            ),
            // Size budget for the overview layer (retail maximum: 512 4bpp tiles).
            (
                "Map2D/global.NCGR".into(),
                Ncgr::new(32, 16, 3, false, vec![0; 32768]).unwrap().write(),
            ),
            (
                "Map2D/global1.NSCR".into(),
                Nscr::new(256, 256, &vec![0; 1024]).unwrap().write(),
            ),
        ]))
        .unwrap()
        .write(),
    );
    for stem in ["studio22", "studio20", "studio20D"] {
        files.insert(
            format!("data/Course/{stem}.carc"),
            Narc::new(BTreeMap::from([
                ("course_model.nsbmd".into(), model.clone()),
                ("course_map.nkm".into(), nitro_formats::synthetic::nkm()),
                (
                    "course_collision.kcl".into(),
                    nitro_formats::synthetic::kcl(),
                ),
            ]))
            .unwrap()
            .write(),
        );
        files.insert(
            format!("data/Course/{stem}Tex.carc"),
            Narc::new(BTreeMap::from([
                ("course_model.nsbtx".into(), tx.clone()),
                (
                    "Map2D/local.NCGR".into(),
                    Ncgr::new(1, 1, 4, false, vec![0; 64]).unwrap().write(),
                ),
                (
                    "Map2D/local.NCLR".into(),
                    Nclr::new(&vec![0; 256], true, false).write(),
                ),
                (
                    "Map2D/local2.NSCR".into(),
                    Nscr::new(8, 8, &[0]).unwrap().write(),
                ),
                (
                    "Map2D/global.NCGR".into(),
                    Ncgr::new(32, 1, 3, false, vec![0; 2048]).unwrap().write(),
                ),
                (
                    "Map2D/global.NCLR".into(),
                    Nclr::new(&[0, 0, 0x3def, 0x7fff], false, false).write(),
                ),
                (
                    "Map2D/global1.NSCR".into(),
                    Nscr::new(256, 256, &vec![0; 1024]).unwrap().write(),
                ),
            ]))
            .unwrap()
            .write(),
        );
    }
    let mut arm = vec![0; 0x180000];
    arm[0x11c..0x124].copy_from_slice(&[0x21, 0x06, 0xc0, 0xde, 0xde, 0xc0, 0x06, 0x21]);
    for id in 0..55 {
        let at = (mkds_data::COURSE_TABLE - mkds_data::ARM9_BASE) as usize + id * 8;
        let string = 0x170000 + id * 32;
        let stem = format!("studio{id}");
        arm[string..string + stem.len()].copy_from_slice(stem.as_bytes());
        put(&mut arm, at, mkds_data::ARM9_BASE + string as u32);
        put(&mut arm, at + 4, u32::from(id == 20));
        if id < 54 {
            let at = (mkds_data::MUSIC_BRANCHES - mkds_data::ARM9_BASE) as usize + id * 4;
            let target = 0x10000 + id * 8;
            put(
                &mut arm,
                at,
                0xea000000 | (((target as i32 - at as i32 - 8) >> 2) as u32 & 0xffffff),
            );
            put(&mut arm, target, 0xe3a0c000);
        }
        for table in [0x16890c, 0x168754] {
            for (k, n) in [-3000i16, -3000, 3000, 3000].iter().enumerate() {
                arm[table + id * 8 + k * 2..table + id * 8 + k * 2 + 2]
                    .copy_from_slice(&n.to_le_bytes());
            }
        }
    }
    for i in 0..32 {
        put(
            &mut arm,
            (mkds_data::CUP_TABLE - mkds_data::ARM9_BASE) as usize + i * 4,
            if i % 2 == 0 { 20 } else { 22 },
        );
    }
    let mut rom = rom_with(files);
    let offset = rom.len().div_ceil(512) * 512;
    rom.resize(offset, 255);
    rom.extend(arm);
    put(&mut rom, 0x20, offset as u32);
    put(&mut rom, 0x28, mkds_data::ARM9_BASE);
    put(&mut rom, 0x2c, 0x180000);
    let len = rom.len();
    put(&mut rom, 0x80, len as u32);
    nitro_formats::rom_build::fix_header_crc(&mut rom);
    rom
}
pub fn rom_with(files: BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let archive = Narc::new(files).expect("synthetic filesystem");
    let a = archive.write();
    let fat_len = u32::from_le_bytes(a[20..24].try_into().expect("FAT length")) as usize;
    let fnt_at = 16 + fat_len;
    let fnt_len =
        u32::from_le_bytes(a[fnt_at + 4..fnt_at + 8].try_into().expect("FNT length")) as usize;
    let fnt = &a[fnt_at + 8..fnt_at + fnt_len];
    let fnt_off = 0x200;
    let fat_off = (fnt_off + fnt.len() + 3) & !3;
    let data_off = fat_off + archive.entries.len() * 8;
    let mut b = vec![0; data_off];
    b[..12].copy_from_slice(b"SYNTHETIC   ");
    b[12..16].copy_from_slice(b"AMCE");
    put(&mut b, 0x40, fnt_off as u32);
    put(&mut b, 0x44, fnt.len() as u32);
    put(&mut b, 0x48, fat_off as u32);
    put(&mut b, 0x4c, (archive.entries.len() * 8) as u32);
    b[fnt_off..fnt_off + fnt.len()].copy_from_slice(fnt);
    for (i, e) in archive.entries.iter().enumerate() {
        let start = b.len() as u32;
        b.extend_from_slice(&e.data);
        let end = b.len() as u32;
        put(&mut b, fat_off + i * 8, start);
        put(&mut b, fat_off + i * 8 + 4, end);
        while !b.len().is_multiple_of(4) {
            b.push(0xff);
        }
    }
    let arm9 = b.len();
    b.extend_from_slice(&[0; 4]);
    put(&mut b, 0x20, arm9 as u32);
    put(&mut b, 0x2c, 4);
    let arm7 = b.len();
    b.extend_from_slice(&[0; 4]);
    put(&mut b, 0x30, arm7 as u32);
    put(&mut b, 0x3c, 4);
    let banner = b.len();
    b.extend_from_slice(&vec![0; 0x840]);
    b[banner] = 1;
    put(&mut b, 0x68, banner as u32);
    b
}

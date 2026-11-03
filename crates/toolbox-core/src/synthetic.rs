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
        ("art/studio.nclr".into(), palette),
        ("art/studio.ncgr".into(), tiles),
        ("art/studio.nscr".into(), screen),
        ("language/en.bmg".into(), text.write()),
    ]))
    .expect("generated archive")
    .write();
    BTreeMap::from([
        (
            "data/Course/studio.carc".into(),
            nitro_formats::compress(
                &Narc::new(BTreeMap::from([
                    (
                        "course_model.nsbmd".into(),
                        nitro_formats::synthetic::nsbmd(),
                    ),
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

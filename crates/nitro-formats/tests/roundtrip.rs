use nitro_formats::{
    archive::Narc,
    bmg::{Bmg, Token},
    graphics::{bgr555_to_rgba, rgba_to_bgr555, Ncgr, Nclr, Nscr},
    texture::{decode, Texture},
};
use std::collections::BTreeMap;
#[test]
fn graphics_roundtrip_and_screen_flips() {
    for depth in [3, 4] {
        for bitmap in [false, true] {
            let palette = Nclr::new(
                &(0..if depth == 3 { 16 } else { 256 }).collect::<Vec<u16>>(),
                depth == 4,
                false,
            );
            let bytes = palette.write();
            assert_eq!(Nclr::parse(&bytes).unwrap().write(), bytes);
            let tiles = Ncgr::new(
                1,
                1,
                depth,
                bitmap,
                (0..64)
                    .map(|i| (i % if depth == 3 { 16 } else { 256 }) as u8)
                    .collect(),
            )
            .unwrap();
            let bytes = tiles.write();
            assert_eq!(Ncgr::parse(&bytes).unwrap().write(), bytes);
            let screen = Nscr::new(8, 8, &[0x400]).unwrap();
            let bytes = screen.write();
            assert_eq!(Nscr::parse(&bytes).unwrap().write(), bytes);
            let image = tiles.render(&palette, Some(&screen), 0).unwrap();
            assert_eq!(&image.rgba[..4], &bgr555_to_rgba(palette.colors[7]));
        }
    }
    let p = Nclr::new(&vec![0x1234; 4096], true, true);
    assert!(Nclr::parse(&p.write()).unwrap().extended);
    for c in 0..32768 {
        assert_eq!(rgba_to_bgr555(bgr555_to_rgba(c)), c);
    }
}
#[test]
fn bmg_preserves_codes_metadata_unknown_sections_and_mid() {
    let mut b = Bmg::new(&["One", "Two 🚀"]);
    b.messages[0].tokens = vec![
        Token::Text(vec![0xd800]),
        Token::Code(vec![26, 0, 6, 2, 0, 7]),
        Token::Text("after".encode_utf16().collect()),
    ];
    let bytes = b.write();
    let parsed = Bmg::parse(&bytes).unwrap();
    assert_eq!(parsed.messages, b.messages);
    assert_eq!(parsed.write(), bytes);
    let mut b = bytes.clone();
    let mut mid = b"MID1\0\0\0\0\x02\0\0\0\0\0\0\0".to_vec();
    mid.extend_from_slice(&42u32.to_le_bytes());
    mid.extend_from_slice(&99u32.to_le_bytes());
    mid.resize(32, 0);
    mid[4..8].copy_from_slice(&32u32.to_le_bytes());
    b.extend_from_slice(&mid);
    let len = b.len() as u32;
    b[8..12].copy_from_slice(&len.to_le_bytes());
    b[12..16].copy_from_slice(&3u32.to_le_bytes());
    let parsed = Bmg::parse(&b).unwrap();
    assert_eq!(parsed.messages[0].id, 42);
    assert_eq!(parsed.messages[1].id, 99);
    assert_eq!(parsed.write(), b);
}
#[test]
fn nested_archives_and_carc_roundtrip() {
    let a = Narc::new(BTreeMap::from([
        (
            "root.nclr".into(),
            Nclr::new(&[31; 16], false, false).write(),
        ),
        (
            "sub/deeper/text.bmg".into(),
            Bmg::new(&["Generated"]).write(),
        ),
    ]))
    .unwrap();
    let bytes = a.write();
    assert_eq!(Narc::parse(&bytes).unwrap().write(), bytes);
    assert_eq!(a.entries[1].name, "sub/deeper/text.bmg");
    let compressed = nitro_formats::compress(&bytes);
    assert_eq!(nitro_formats::decompress(&compressed).unwrap(), bytes);
    assert_eq!(nitro_formats::recompress(&compressed, &bytes), compressed);
    let mut edited = a.clone();
    edited.entries[1].data = Bmg::new(&["Longer generated message"]).write();
    let new = edited.write();
    let packed = nitro_formats::recompress(&compressed, &new);
    assert_eq!(nitro_formats::decompress(&packed).unwrap(), new);
}
#[test]
fn every_texture_format_and_compression_mode() {
    let palette = (0..256).map(|i| i as u16).collect::<Vec<_>>();
    for format in 1..=7 {
        for mode in 0..4 {
            let len = match format {
                2 | 5 => 16,
                3 => 32,
                7 => 128,
                _ => 64,
            };
            let mut t = Texture {
                name: "synthetic".into(),
                width: 8,
                height: 8,
                format,
                transparent_zero: true,
                data: vec![0; len],
                indices: vec![0; 8],
            };
            for pair in t.indices.chunks_exact_mut(2) {
                pair.copy_from_slice(&((mode as u16) << 14).to_le_bytes());
            }
            let image = decode(&t, &palette).unwrap();
            assert_eq!(image.rgba.len(), 256);
            if format == 7 {
                assert_eq!(image.rgba[3], 0);
            } else if format == 5 {
                assert_eq!(image.rgba[3], 255);
            } else {
                assert_eq!(image.rgba[3], 0);
            }
        }
    }
}

#[test]
fn texture_pixels_obey_packed_indices_and_alpha() {
    let palette = vec![0, 31, 31 << 5, 31 << 10];
    let t = Texture {
        name: "indices".into(),
        width: 8,
        height: 8,
        format: 2,
        transparent_zero: true,
        data: vec![0xe4; 16],
        indices: vec![],
    };
    let image = decode(&t, &palette).unwrap();
    assert_eq!(
        &image.rgba[..16],
        &[0, 0, 0, 0, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255]
    );
    let mut t = Texture {
        format: 1,
        data: vec![0xff; 64],
        ..t
    };
    let colors = vec![31; 32];
    assert_eq!(&decode(&t, &colors).unwrap().rgba[..4], &[255, 0, 0, 255]);
    t.format = 6;
    t.data.fill(0x88);
    assert_eq!(decode(&t, &colors).unwrap().rgba[3], 139);
    t.format = 7;
    t.data = vec![0xff; 128];
    assert_eq!(&decode(&t, &[]).unwrap().rgba[..4], &[255, 255, 255, 255]);
    t.format = 5;
    t.data = vec![0xff; 16];
    t.indices = vec![0; 8];
    assert_eq!(decode(&t, &palette).unwrap().rgba[3], 0);
    for mode in 0..4u16 {
        for d in t.indices.chunks_exact_mut(2) {
            d.copy_from_slice(&(mode << 14).to_le_bytes());
        }
        assert_eq!(
            decode(&t, &palette).unwrap().rgba[3],
            if mode < 2 { 0 } else { 255 }
        );
    }
}
#[test]
fn lz_splices_small_edits_and_reports_bad_data() {
    let plain = (0..10000)
        .flat_map(|i| format!("synthetic {} value {}!", i % 32, i % 7).into_bytes())
        .collect::<Vec<_>>();
    let c = nitro_formats::compress(&plain);
    let mut edited = plain.clone();
    edited[2000] ^= 1;
    let n = nitro_formats::recompress(&c, &edited);
    assert_eq!(nitro_formats::decompress(&n).unwrap(), edited);
    let mut patch = vec![];
    qbsdiff::Bsdiff::new(&c, &n).compare(&mut patch).unwrap();
    assert!(patch.len() < 1000);
    assert!(nitro_formats::decompress(&[16, 4, 0, 0, 128, 0, 1]).is_err());
}
#[test]
fn parsers_never_panic_on_truncations_and_noise() {
    let fixtures = vec![
        Nclr::new(&[0; 16], false, false).write(),
        Ncgr::new(1, 1, 3, false, vec![0; 64]).unwrap().write(),
        Nscr::new(8, 8, &[0]).unwrap().write(),
        Bmg::new(&["synthetic"]).write(),
        Narc::new(BTreeMap::from([(
            "a.bmg".into(),
            Bmg::new(&["test"]).write(),
        )]))
        .unwrap()
        .write(),
    ];
    let check = |b: &[u8]| {
        let _ = Nclr::parse(b);
        let _ = Ncgr::parse(b);
        let _ = Nscr::parse(b);
        let _ = Bmg::parse(b);
        let _ = Narc::parse(b);
        let _ = nitro_formats::rom::Rom::parse(b);
        let _ = nitro_formats::texture::Nsbtx::parse(b);
        let _ = nitro_formats::decompress(b);
    };
    for f in fixtures {
        for i in 0..f.len() {
            check(&f[..i]);
        }
        for offset in 0..f.len() {
            let mut bytes = f.clone();
            bytes[offset] = 255;
            check(&bytes);
        }
    }
    let mut seed = 71u32;
    for len in 0..512 {
        let bytes = (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 24) as u8
            })
            .collect::<Vec<_>>();
        check(&bytes);
    }
}

#[test]
fn archive_relocation_preserves_unknown_blocks_and_padding() {
    let a = Narc::new(BTreeMap::from([
        ("a.bmg".into(), b"first".to_vec()),
        ("sub/b.bmg".into(), b"second".to_vec()),
    ]))
    .unwrap();
    let mut bytes = a.write();
    let tail = b"XTRA\x0c\0\0\0keep";
    bytes.extend_from_slice(tail);
    bytes[14..16].copy_from_slice(&4u16.to_le_bytes());
    let declared = bytes.len() as u32;
    bytes[8..12].copy_from_slice(&declared.to_le_bytes());
    bytes.extend_from_slice(b"trailing padding");
    let mut a = Narc::parse(&bytes).unwrap();
    assert_eq!(a.write(), bytes);
    a.entries[0].data = b"FIRST".to_vec();
    let splice = a.write();
    assert_eq!(splice.len(), bytes.len());
    assert_eq!(splice.iter().zip(&bytes).filter(|(a, b)| a != b).count(), 5);
    a.entries[0].data = b"a much longer authored replacement".to_vec();
    let grown = a.write();
    assert!(grown.ends_with(b"XTRA\x0c\0\0\0keeptrailing padding"));
    let reparsed = Narc::parse(&grown).unwrap();
    assert_eq!(reparsed.entries[0].data, a.entries[0].data);
    assert_eq!(reparsed.entries[1].data, b"second");
    assert_eq!(reparsed.entries[1].name, "sub/b.bmg");
    assert_eq!(reparsed.write(), grown);
}
#[test]
fn detection_and_texture_decode_bound_malformed_inputs() {
    let mut nested = Nclr::new(&[0; 16], false, false).write();
    for _ in 0..32 {
        nested = nitro_formats::compress(&nested);
    }
    assert!(nitro_formats::detect::detect(&nested, "x.nclr").is_none());
    let t = Texture {
        name: String::new(),
        width: 1,
        height: 16,
        format: 5,
        transparent_zero: false,
        data: vec![0; 4],
        indices: vec![0; 2],
    };
    assert!(decode(&t, &[0; 4]).is_err());
}

//! Generated, original geometry for tests and browser demos. Contains no game data.
use crate::{put16, put32};
pub fn dictionary(entries: &[(&str, Vec<u8>)], stride: usize) -> Vec<u8> {
    let table = 8 + (entries.len() + 1) * 4;
    let names = table + 4 + entries.len() * stride;
    let len = names + entries.len() * 16;
    let mut b = vec![0; len];
    b[1] = entries.len() as u8;
    put16(&mut b, 2, len as u16);
    put16(&mut b, 4, 8);
    put16(&mut b, 6, (table - 8) as u16);
    put16(&mut b, table, stride as u16);
    put16(&mut b, table + 2, (names - table) as u16);
    for (i, (name, d)) in entries.iter().enumerate() {
        b[table + 4 + i * stride..table + 4 + (i + 1) * stride].copy_from_slice(d);
        b[names + i * 16..names + i * 16 + name.len()].copy_from_slice(name.as_bytes())
    }
    b
}
pub fn container(magic: &[u8; 4], block: Vec<u8>) -> Vec<u8> {
    let mut b = vec![0; 20];
    b[..4].copy_from_slice(magic);
    put16(&mut b, 4, 0xfeff);
    put16(&mut b, 6, 1);
    put16(&mut b, 12, 16);
    put16(&mut b, 14, 1);
    put32(&mut b, 16, 20);
    b.extend(block);
    let n = b.len() as u32;
    put32(&mut b, 8, n);
    b
}
fn gx(commands: &[(u8, Vec<u32>)]) -> Vec<u8> {
    let mut b = vec![];
    for c in commands.chunks(4) {
        let mut op = [0; 4];
        for (i, (v, _)) in c.iter().enumerate() {
            op[i] = *v
        }
        b.extend(op);
        for (_, p) in c {
            for w in p {
                b.extend(w.to_le_bytes())
            }
        }
    }
    b
}
pub fn nsbmd() -> Vec<u8> {
    let mut cmds = vec![
        (0x40, vec![1]),
        (0x21, vec![511 << 10]),
        (0x20, vec![0x7fff]),
    ];
    for (x, y, z, u, v) in [
        (-3., 0., -3., 0, 0),
        (3., 0., -3., 8, 0),
        (3., 0., 3., 8, 8),
        (-3., 0., 3., 0, 8),
        (-1., 0., -1., 0, 0),
        (-1., 2., -1., 0, 8),
        (1., 2., -1., 8, 8),
        (1., 0., -1., 8, 0),
    ] {
        cmds.push((0x22, vec![(u * 16) | ((v * 16) << 16)]));
        let f = |x: f32| (x * 4096.) as i16 as u16 as u32;
        cmds.push((0x23, vec![f(x) | (f(y) << 16), f(z)]));
    }
    cmds.push((0x41, vec![]));
    let commands = gx(&cmds);
    let node_dict_len = dictionary(&[("root", vec![0; 4])], 4).len();
    let nodes = dictionary(
        &[("root", (node_dict_len as u32).to_le_bytes().to_vec())],
        4,
    );
    let mut b = vec![0; 64];
    b.extend(nodes);
    b.extend([7, 0, 0, 0]);
    let sbc = b.len();
    b.extend([0x26, 0, 0, 0, 0, 0x0b, 4, 0, 5, 0, 1]);
    while b.len() % 4 != 0 {
        b.push(0)
    }
    let mats = b.len();
    b.extend([0; 4]);
    let dict_len = dictionary(&[("paint", vec![0; 4])], 4).len();
    b.extend(dictionary(
        &[("paint", ((4 + dict_len) as u32).to_le_bytes().to_vec())],
        4,
    ));
    let mut mat = vec![0; 44];
    put16(&mut mat, 2, 44);
    put32(&mut mat, 4, 0x7fff);
    put32(&mut mat, 12, 0x001f00c0);
    put16(&mut mat, 32, 8);
    put16(&mut mat, 34, 8);
    b.extend(mat);
    let texoff = b.len() - mats;
    let pair_len = dictionary(&[("studio", vec![0; 4])], 4).len();
    let paloff = texoff + pair_len;
    let pal = dictionary(&[], 4);
    let ids = paloff + pal.len();
    let mut p = vec![0; 4];
    put16(&mut p, 0, ids as u16);
    p[2] = 1;
    b.extend(dictionary(&[("studio", p)], 4));
    b.extend(pal);
    b.push(0);
    while b.len() % 4 != 0 {
        b.push(0)
    }
    put16(&mut b, mats, texoff as u16);
    put16(&mut b, mats + 2, paloff as u16);
    let shapes = b.len();
    let size = dictionary(&[("platform", vec![0; 4])], 4).len();
    b.extend(dictionary(
        &[("platform", (size as u32).to_le_bytes().to_vec())],
        4,
    ));
    let mut shape = vec![0; 16];
    put16(&mut shape, 2, 16);
    put32(&mut shape, 8, 16);
    put32(&mut shape, 12, commands.len() as u32);
    b.extend(shape);
    b.extend(commands);
    let len = b.len() as u32;
    put32(&mut b, 0, len);
    put32(&mut b, 4, sbc as u32);
    put32(&mut b, 8, mats as u32);
    put32(&mut b, 12, shapes as u32);
    put32(&mut b, 16, len);
    b[23] = 1;
    b[24] = 1;
    b[25] = 1;
    put32(&mut b, 32, 128 * 4096);
    put32(&mut b, 36, 32);
    put16(&mut b, 40, 8);
    put16(&mut b, 42, 2);
    put16(&mut b, 46, 2);
    let size = dictionary(&[("studio", vec![0; 4])], 4).len();
    let mut mdl = b"MDL0\0\0\0\0".to_vec();
    mdl.extend(dictionary(
        &[("studio", ((8 + size) as u32).to_le_bytes().to_vec())],
        4,
    ));
    mdl.extend(b);
    let len = mdl.len() as u32;
    put32(&mut mdl, 4, len);
    container(b"BMD0", mdl)
}
pub fn nsbca() -> Vec<u8> {
    let mut b = b"J\0AC".to_vec();
    b.resize(22, 0);
    put16(&mut b, 4, 60);
    put16(&mut b, 6, 1);
    put16(&mut b, 20, 22);
    b.extend(0x0730u16.to_le_bytes());
    b.extend([0, 0]);
    b.extend((60u32 << 16).to_le_bytes());
    b.extend(52u32.to_le_bytes());
    b.extend([0; 8]);
    b.extend(0x8000u32.to_le_bytes());
    let pivot = b.len() as u32;
    put32(&mut b, 12, pivot);
    b.extend([0, 0, 0, 16, 0, 0]);
    assert_eq!(b.len(), 52);
    for i in 0..60 {
        let x = ((i as f32 / 60. * std::f32::consts::TAU).sin() * 4096.) as i32;
        b.extend(x.to_le_bytes())
    }
    let size = dictionary(&[("Float", vec![0; 4])], 4).len();
    let mut j = b"JNT0\0\0\0\0".to_vec();
    j.extend(dictionary(
        &[("Float", ((8 + size) as u32).to_le_bytes().to_vec())],
        4,
    ));
    j.extend(b);
    let len = j.len() as u32;
    put32(&mut j, 4, len);
    container(b"BCA0", j)
}
fn record(kind: &str, p: [f32; 3]) -> Vec<u8> {
    let mut b = vec![0; crate::nkm::stride(kind, 37).unwrap()];
    if b.len() >= 12 {
        for (i, v) in p.into_iter().enumerate() {
            put32(&mut b, i * 4, (v * 4096.) as i32 as u32)
        }
    }
    b
}
pub fn nkm() -> Vec<u8> {
    let mut obj = record("OBJI", [160., 0., 80.]);
    for i in 0..3 {
        put32(&mut obj, 24 + i * 4, 4096)
    }
    put16(&mut obj, 36, 42);
    put16(&mut obj, 38, 65535);
    put32(&mut obj, 56, 1);
    let points = vec![
        [-250., 10., -250.],
        [250., 10., -250.],
        [250., 10., 250.],
        [-250., 10., 250.],
    ];
    let path = |kind: &str| {
        let mut p = record(kind, [0.; 3]);
        put16(&mut p, 2, 4);
        p[4] = 0;
        p[5] = 255;
        p[6] = 255;
        p[7] = 0;
        p[8] = 255;
        p[9] = 255;
        p
    };
    let mut gate = vec![0; 36];
    for (i, v) in [-280., -180., 280., -180.].into_iter().enumerate() {
        put32(&mut gate, i * 4, (v * 4096.) as i32 as u32)
    }
    put16(&mut gate, 28, 65535);
    put16(&mut gate, 30, 65535);
    put16(&mut gate, 32, 65535);
    let mut area = record("AREA", [0., 0., 160.]);
    for i in 0..3 {
        put32(&mut area, 12 + i * 4, 4096);
        put32(&mut area, 24 + i * 16, 4096)
    }
    let sections = vec![
        ("OBJI", vec![obj]),
        ("PATH", vec![vec![0, 1, 4, 0]]),
        ("POIT", points.iter().map(|p| record("POIT", *p)).collect()),
        ("STAG", vec![vec![0; 40]]),
        ("KTPS", vec![record("KTPS", [0., 10., -220.])]),
        ("KTPJ", vec![record("KTPJ", [0., 10., -200.])]),
        ("KTP2", vec![]),
        ("KTPC", vec![]),
        ("KTPM", vec![]),
        ("CPOI", vec![gate]),
        (
            "CPAT",
            vec![{
                let mut p = path("CPAT");
                put16(&mut p, 2, 1);
                p
            }],
        ),
        (
            "IPOI",
            points
                .iter()
                .map(|p| record("IPOI", [p[0] * 0.8, 20., p[2] * 0.8]))
                .collect(),
        ),
        ("IPAT", vec![path("IPAT")]),
        ("EPOI", points.iter().map(|p| record("EPOI", *p)).collect()),
        ("EPAT", vec![path("EPAT")]),
        ("MEPO", vec![]),
        ("MEPA", vec![]),
        ("AREA", vec![area]),
        ("CAME", vec![record("CAME", [-100., 180., 180.])]),
    ];
    crate::nkm::Nkm::new(&sections).unwrap().write().unwrap()
}
pub fn kcl() -> Vec<u8> {
    // Two triangles, with a +Y face normal and outward edge normals.
    let mut b = vec![0; 60];
    put32(&mut b, 0, 60);
    put32(&mut b, 4, 84);
    put32(&mut b, 8, 116);
    put32(&mut b, 12, 164);
    put32(&mut b, 16, 30 * 4096);
    for i in 0..3 {
        put32(&mut b, 32 + i * 4, 0xfffffc00)
    }
    put32(&mut b, 44, 10);
    put32(&mut b, 56, 25 * 4096);
    for p in [[-384., 0., -384.], [384., 0., 384.]] {
        for v in p {
            b.extend(((v * 4096.) as i32).to_le_bytes())
        }
    }
    for n in [
        [0., 1., 0.],
        [-1., 0., 0.],
        [0., 0., -1.],
        [0.70703125, 0., 0.70703125],
        [1., 0., 0.],
        [0., 0., 1.],
        [-0.70703125, 0., -0.70703125],
        [0., 1., 0.],
    ] {
        for v in n {
            b.extend(((v * 4096.) as i16).to_le_bytes())
        }
    }
    for (p, n, a) in [(0, [0, 1, 2, 3], 0), (1, [0, 4, 5, 6], 1)] {
        b.extend((543u32 * 4096).to_le_bytes());
        b.extend((p as u16).to_le_bytes());
        for i in n {
            b.extend((i as u16).to_le_bytes())
        }
        b.extend((a as u16).to_le_bytes())
    }
    b.extend(0x80000002u32.to_le_bytes());
    b.extend([1, 0, 2, 0, 0, 0]);
    b
}

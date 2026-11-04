//! Independent NCER/NANR readers. Format reference: Gonhex's CEBK/ABNK research,
//! https://ds-pokemon-hacking.github.io/docs/universal/resources/nitro/graphics_2d/section_cebk/
//! and section_abnk/. No third-party implementation code is copied.
use crate::{
    error,
    graphics::{bgr555_to_rgba, blocks, Image, Ncgr, Nclr},
    slice, u16le, u32le, Result,
};
#[derive(Clone, Debug)]
pub struct Cell {
    pub objects: Vec<[u16; 3]>,
}
#[derive(Clone, Debug)]
pub struct Ncer {
    raw: Vec<u8>,
    pub cells: Vec<Cell>,
    pub mapping: u32,
}
impl Ncer {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let (_, b) = blocks(bytes, b"RECN")?
            .into_iter()
            .find(|(_, b)| b.starts_with(b"KBEC"))
            .ok_or_else(|| error(16, "missing cell bank"))?;
        let n = u16le(b, 8)? as usize;
        let bounds = u16le(b, 10)?;
        if bounds > 1 {
            return Err(error(10, "unsupported cell bounds flag"));
        }
        let stride = 8 << bounds;
        let table = 8 + u32le(b, 12)? as usize;
        slice(b, table, n * stride)?;
        let attrs = table + n * stride;
        let mapping = u32le(b, 16)?;
        if mapping > 4 {
            return Err(error(16, "unsupported cell mapping"));
        }
        let mut cells = vec![];
        for i in 0..n {
            let at = table + i * stride;
            let count = u16le(b, at)? as usize;
            let base = attrs + u32le(b, at + 4)? as usize;
            slice(b, base, count * 6)?;
            let mut objects = vec![];
            for j in 0..count {
                let p = base + j * 6;
                objects.push([u16le(b, p)?, u16le(b, p + 2)?, u16le(b, p + 4)?]);
            }
            cells.push(Cell { objects });
        }
        Ok(Self {
            raw: bytes.to_vec(),
            cells,
            mapping,
        })
    }
    pub fn write(&self) -> Vec<u8> {
        self.raw.clone()
    }
    pub fn render(&self, index: usize, tiles: &Ncgr, palette: &Nclr) -> Result<Image> {
        let cell = self
            .cells
            .get(index)
            .ok_or_else(|| error(index, "cell index out of bounds"))?;
        let mut rgba = vec![0; 512 * 256 * 4];
        let mut objects: Vec<_> = cell.objects.iter().enumerate().collect();
        objects.sort_by_key(|(i, a)| (std::cmp::Reverse((a[2] >> 10) & 3), std::cmp::Reverse(*i)));
        for (_, [a, b, c]) in objects {
            if a & 0x300 == 0x200 {
                continue;
            }
            let shape = a >> 14;
            let size = b >> 14;
            let dims = [
                [(8, 8), (16, 16), (32, 32), (64, 64)],
                [(16, 8), (32, 8), (32, 16), (64, 32)],
                [(8, 16), (8, 32), (16, 32), (32, 64)],
            ];
            if shape >= 3 {
                return Err(error(0, "invalid OAM shape"));
            }
            let (w, h) = dims[shape as usize][size as usize];
            let x = (((b & 511) as i16) << 7 >> 7) as i32;
            let y = (a & 255) as u8 as i8 as i32;
            let eight = a & 0x2000 != 0;
            let base = (c & 1023) as usize
                * (1usize << if self.mapping == 4 { 0 } else { self.mapping })
                * 64
                / if eight { 2 } else { 1 };
            for py in 0..h {
                for px in 0..w {
                    let sx = if a & 0x100 == 0 && b & 0x1000 != 0 {
                        w - 1 - px
                    } else {
                        px
                    };
                    let sy = if a & 0x100 == 0 && b & 0x2000 != 0 {
                        h - 1 - py
                    } else {
                        py
                    };
                    let tile = (sy / 8)
                        * if self.mapping == 4 {
                            if eight {
                                16
                            } else {
                                32
                            }
                        } else {
                            w / 8
                        }
                        + sx / 8;
                    let index = *tiles
                        .pixels
                        .get(base + tile * 64 + (sy % 8) * 8 + sx % 8)
                        .ok_or_else(|| error(base, "cell tile outside NCGR"))?
                        as usize;
                    if index == 0 {
                        continue;
                    }
                    let bank = if eight { 0 } else { (c >> 12) as usize * 16 };
                    let color = *palette
                        .colors
                        .get(bank + index)
                        .ok_or_else(|| error(index, "cell palette outside NCLR"))?;
                    let dx = x + px as i32 + 256;
                    let dy = y + py as i32 + 128;
                    if (0..512).contains(&dx) && (0..256).contains(&dy) {
                        let dest = (dy as usize * 512 + dx as usize) * 4;
                        rgba[dest..dest + 4].copy_from_slice(&bgr555_to_rgba(color));
                    }
                }
            }
        }
        Ok(Image {
            width: 512,
            height: 256,
            rgba,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub cell: u16,
    pub duration: u16,
    pub translation: [i16; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
}
#[derive(Clone, Debug)]
pub struct Sequence {
    pub frames: Vec<Frame>,
    pub loop_mode: u32,
    pub start_frame: u16,
}
#[derive(Clone, Debug)]
pub struct Nanr {
    raw: Vec<u8>,
    pub sequences: Vec<Sequence>,
}
impl Nanr {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let (_, b) = blocks(bytes, b"RNAN")?
            .into_iter()
            .find(|(_, b)| b.starts_with(b"KNBA"))
            .ok_or_else(|| error(16, "missing animation bank"))?;
        let n = u16le(b, 8)? as usize;
        let seq = 8 + u32le(b, 12)? as usize;
        let frames = 8 + u32le(b, 16)? as usize;
        let props = 8 + u32le(b, 20)? as usize;
        slice(b, seq, n * 16)?;
        slice(b, frames, u16le(b, 10)? as usize * 8)?;
        let mut sequences = vec![];
        for i in 0..n {
            let at = seq + i * 16;
            let count = u16le(b, at)? as usize;
            let start_frame = u16le(b, at + 2)?;
            let kind = u16le(b, at + 4)?;
            let cell_type = u16le(b, at + 6)?;
            if kind > 2 || cell_type != 1 {
                return Err(error(at + 4, "unsupported animation or multi-cell type"));
            }
            let first = frames + u32le(b, at + 12)? as usize;
            slice(b, first, count * 8)?;
            let mut list = vec![];
            for j in 0..count {
                let f = first + j * 8;
                let p = props + u32le(b, f)? as usize;
                let len = match kind {
                    0 => 2,
                    1 => 16,
                    _ => 8,
                };
                slice(b, p, len)?;
                let mut frame = Frame {
                    cell: u16le(b, p)?,
                    duration: u16le(b, f + 4)?.max(1),
                    translation: [0, 0],
                    scale: [1., 1.],
                    rotation: 0.,
                };
                if kind == 1 {
                    frame.rotation = u16le(b, p + 2)? as f32 * std::f32::consts::TAU / 65536.;
                    frame.scale = [
                        u32le(b, p + 4)? as i32 as f32 / 4096.,
                        u32le(b, p + 8)? as i32 as f32 / 4096.,
                    ];
                    frame.translation = [u16le(b, p + 12)? as i16, u16le(b, p + 14)? as i16];
                }
                if kind == 2 {
                    frame.translation = [u16le(b, p + 4)? as i16, u16le(b, p + 6)? as i16];
                }
                list.push(frame);
            }
            sequences.push(Sequence {
                frames: list,
                loop_mode: u32le(b, at + 8)?,
                start_frame,
            });
        }
        Ok(Self {
            raw: bytes.to_vec(),
            sequences,
        })
    }
    pub fn write(&self) -> Vec<u8> {
        self.raw.clone()
    }
}
pub fn transform(image: &Image, frame: &Frame) -> Image {
    let mut rgba = vec![0; image.rgba.len()];
    let (sin, cos) = frame.rotation.sin_cos();
    for y in 0..image.height {
        for x in 0..image.width {
            let dx = x as f32 - image.width as f32 / 2. - frame.translation[0] as f32;
            let dy = y as f32 - image.height as f32 / 2. - frame.translation[1] as f32;
            let sx = ((dx * cos + dy * sin) / frame.scale[0] + image.width as f32 / 2.).round();
            let sy = ((-dx * sin + dy * cos) / frame.scale[1] + image.height as f32 / 2.).round();
            if sx.is_finite()
                && sy.is_finite()
                && sx >= 0.
                && sy >= 0.
                && sx < image.width as f32
                && sy < image.height as f32
            {
                let dest = (y * image.width + x) as usize * 4;
                let src = (sy as u32 * image.width + sx as u32) as usize * 4;
                rgba[dest..dest + 4].copy_from_slice(&image.rgba[src..src + 4]);
            }
        }
    }
    Image {
        width: image.width,
        height: image.height,
        rgba,
    }
}

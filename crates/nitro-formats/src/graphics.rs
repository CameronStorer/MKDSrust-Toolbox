use crate::{error, put16, put32, slice, u16le, u32le, Result};

pub fn bgr555_to_rgba(c: u16) -> [u8; 4] {
    let expand = |v: u16| ((v << 3) | (v >> 2)) as u8;
    [
        expand(c & 31),
        expand((c >> 5) & 31),
        expand((c >> 10) & 31),
        255,
    ]
}
pub fn rgba_to_bgr555(c: [u8; 4]) -> u16 {
    (c[0] as u16 >> 3) | ((c[1] as u16 >> 3) << 5) | ((c[2] as u16 >> 3) << 10)
}

pub(crate) fn blocks<'a>(b: &'a [u8], magic: &[u8; 4]) -> Result<Vec<(usize, &'a [u8])>> {
    if slice(b, 0, 4)? != magic {
        return Err(error(0, "unexpected file magic"));
    }
    // Early Nitro character exporters (including the debug font) use zero.
    if ![0, 0xfeff, 0xfffe].contains(&u16le(b, 4)?) {
        return Err(error(4, "only little-endian Nitro files supported"));
    }
    let end = u32le(b, 8)? as usize;
    if end > b.len() || end < 16 {
        return Err(error(8, "invalid file size"));
    }
    let mut p = u16le(b, 12)? as usize;
    if p < 16 {
        return Err(error(12, "invalid header size"));
    }
    let mut out = vec![];
    for _ in 0..u16le(b, 14)? {
        let len = u32le(b, p + 4)? as usize;
        if len < 8 || p + len > end {
            return Err(error(p + 4, "invalid block length"));
        }
        out.push((p, slice(b, p, len)?));
        p += len;
    }
    Ok(out)
}
fn make(magic: &[u8; 4], tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut b = magic.to_vec();
    b.extend_from_slice(&[0xff, 0xfe, 0, 1]);
    b.extend_from_slice(&((24 + body.len()) as u32).to_le_bytes());
    b.extend_from_slice(&[16, 0, 1, 0]);
    b.extend_from_slice(tag);
    b.extend_from_slice(&((body.len() + 8) as u32).to_le_bytes());
    b.extend_from_slice(body);
    b
}

#[derive(Clone, Debug)]
pub struct Nclr {
    raw: Vec<u8>,
    color_at: usize,
    pub depth: u32,
    pub extended: bool,
    pub colors: Vec<u16>,
}
impl Nclr {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let sections = blocks(b, b"RLCN")?;
        let compressed = sections.iter().any(|(_, s)| s.starts_with(b"PMCP"));
        let (at, s) = sections
            .into_iter()
            .find(|(_, s)| s.starts_with(b"TTLP"))
            .ok_or_else(|| error(16, "missing TTLP"))?;
        let depth = u32le(s, 8)?;
        if depth != 3 && depth != 4 {
            return Err(error(at + 8, "palette depth must be 3 (4bpp) or 4 (8bpp)"));
        }
        let len = u32le(s, 16)? as usize;
        let rel = 8 + u32le(s, 20)? as usize;
        if !len.is_multiple_of(2) || rel < 24 {
            return Err(error(at + 16, "invalid palette data"));
        }
        // PCMP stores only the used palette slots. TTLP's size may describe
        // the expanded palette address space rather than the stored colours.
        let stored = if compressed {
            len.min(s.len().saturating_sub(rel))
        } else {
            len
        };
        let data = slice(s, rel, stored)?;
        Ok(Self {
            raw: b.to_vec(),
            color_at: at + rel,
            depth,
            extended: u32le(s, 12)? != 0,
            colors: data
                .chunks_exact(2)
                .map(|x| u16::from_le_bytes([x[0], x[1]]))
                .collect(),
        })
    }
    pub fn new(colors: &[u16], eight_bit: bool, extended: bool) -> Self {
        let mut body = vec![0; 16 + colors.len() * 2];
        put32(&mut body, 0, if eight_bit { 4 } else { 3 });
        put32(&mut body, 4, u32::from(extended));
        put32(&mut body, 8, (colors.len() * 2) as u32);
        put32(&mut body, 12, 16);
        for (i, c) in colors.iter().enumerate() {
            put16(&mut body, 16 + i * 2, *c);
        }
        Self {
            raw: make(b"RLCN", b"TTLP", &body),
            color_at: 40,
            depth: if eight_bit { 4 } else { 3 },
            extended,
            colors: colors.to_vec(),
        }
    }
    pub fn write(&self) -> Vec<u8> {
        if self.color_at + self.colors.len() * 2 > self.raw.len() {
            return Self::new(&self.colors, self.depth == 4, self.extended).raw;
        }
        let mut out = self.raw.clone();
        for (i, c) in self.colors.iter().enumerate() {
            put16(&mut out, self.color_at + i * 2, *c);
        }
        out
    }
}

#[derive(Clone, Debug)]
pub struct Ncgr {
    raw: Vec<u8>,
    pub width_tiles: u16,
    pub height_tiles: u16,
    pub depth: u32,
    pub bitmap: bool,
    pub pixels: Vec<u8>,
    pixel_at: usize,
}
impl Ncgr {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let (at, s) = blocks(b, b"RGCN")?
            .into_iter()
            .find(|(_, s)| s.starts_with(b"RAHC"))
            .ok_or_else(|| error(16, "missing RAHC"))?;
        let depth = u32le(s, 12)?;
        if ![3, 4].contains(&depth) {
            return Err(error(at + 12, "invalid tile depth"));
        }
        let size = u32le(s, 24)? as usize;
        let rel = 8 + u32le(s, 28)? as usize;
        if rel < 32 || !size.is_multiple_of(if depth == 3 { 32 } else { 64 }) {
            return Err(error(at + 24, "invalid tile data size"));
        }
        // Legacy zero-BOM single-block exports undercount both lengths by
        // the eight-byte chunk header. Read only the bounded trailing data.
        let legacy = u16le(b, 4)? == 0
            && u16le(b, 14)? == 1
            && b.len() == u32le(b, 8)? as usize + 8
            && rel + size == s.len() + 8;
        let data = if legacy {
            slice(b, at + rel, size)?
        } else {
            slice(s, rel, size)?
        };
        let pixels = if depth == 3 {
            data.iter().flat_map(|x| [x & 15, x >> 4]).collect()
        } else {
            data.to_vec()
        };
        Ok(Self {
            raw: b.to_vec(),
            width_tiles: u16le(s, 10)?,
            height_tiles: u16le(s, 8)?,
            depth,
            bitmap: u32le(s, 20)? & 255 == 1,
            pixels,
            pixel_at: at + rel,
        })
    }
    pub fn new(
        width_tiles: u16,
        height_tiles: u16,
        depth: u32,
        bitmap: bool,
        pixels: Vec<u8>,
    ) -> Result<Self> {
        if ![3, 4].contains(&depth) || !pixels.len().is_multiple_of(64) {
            return Err(error(0, "invalid tiles"));
        }
        let data = if depth == 3 {
            pixels
                .chunks_exact(2)
                .map(|p| p[0] & 15 | (p[1] & 15) << 4)
                .collect::<Vec<_>>()
        } else {
            pixels
        };
        let mut body = vec![0; 24];
        put16(&mut body, 0, height_tiles);
        put16(&mut body, 2, width_tiles);
        put32(&mut body, 4, depth);
        put32(&mut body, 12, u32::from(bitmap));
        put32(&mut body, 16, data.len() as u32);
        put32(&mut body, 20, 24);
        body.extend_from_slice(&data);
        Self::parse(&make(b"RGCN", b"RAHC", &body))
    }
    pub fn write(&self) -> Vec<u8> {
        let mut b = self.raw.clone();
        let data = if self.depth == 3 {
            self.pixels
                .chunks_exact(2)
                .map(|p| p[0] & 15 | (p[1] & 15) << 4)
                .collect::<Vec<_>>()
        } else {
            self.pixels.clone()
        };
        if let Some(dest) = b.get_mut(self.pixel_at..self.pixel_at + data.len()) {
            dest.copy_from_slice(&data);
        }
        b
    }
    pub fn render(&self, palette: &Nclr, screen: Option<&Nscr>, bank: usize) -> Result<Image> {
        let (width, height) = if let Some(s) = screen {
            (s.width as usize, s.height as usize)
        } else {
            let w = if self.width_tiles == 0 || self.width_tiles == 65535 {
                8
            } else {
                self.width_tiles as usize
            };
            (w * 8, ((self.pixels.len() / 64).div_ceil(w)) * 8)
        };
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(error(0, "image dimensions outside preview limits"));
        }
        let mut rgba = vec![0; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let (tile, px, py, pal) = if let Some(s) = screen {
                    let e = *s
                        .entries
                        .get((y / 8) * (width / 8) + x / 8)
                        .ok_or_else(|| error(0, "tile map too short"))?;
                    (
                        (e & 1023) as usize,
                        if e & 1024 != 0 { 7 - x % 8 } else { x % 8 },
                        if e & 2048 != 0 { 7 - y % 8 } else { y % 8 },
                        (e >> 12) as usize,
                    )
                } else {
                    ((y / 8) * (width / 8) + x / 8, x % 8, y % 8, bank)
                };
                let index = if self.bitmap && screen.is_none() {
                    y * width + x
                } else {
                    tile * 64 + py * 8 + px
                };
                let Some(&pixel) = self.pixels.get(index) else {
                    continue;
                };
                let color = pixel as usize
                    + if self.depth == 3 {
                        pal * 16
                    } else {
                        bank * 256
                    };
                if let Some(c) = palette.colors.get(color) {
                    let mut c = bgr555_to_rgba(*c);
                    if pixel == 0 {
                        c[3] = 0;
                    }
                    rgba[(y * width + x) * 4..(y * width + x + 1) * 4].copy_from_slice(&c);
                }
            }
        }
        Ok(Image {
            width: width as u32,
            height: height as u32,
            rgba,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Nscr {
    raw: Vec<u8>,
    entry_at: usize,
    pub width: u16,
    pub height: u16,
    pub entries: Vec<u16>,
}
impl Nscr {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let (at, s) = blocks(b, b"RCSN")?
            .into_iter()
            .find(|(_, s)| s.starts_with(b"NRCS"))
            .ok_or_else(|| error(16, "missing NRCS"))?;
        let width = u16le(s, 8)?;
        let height = u16le(s, 10)?;
        let len = u32le(s, 16)? as usize;
        if width == 0
            || height == 0
            || width % 8 != 0
            || height % 8 != 0
            || !len.is_multiple_of(2)
            || len < width as usize * height as usize / 32
        {
            return Err(error(at + 8, "invalid screen dimensions or entries"));
        }
        let entries = slice(s, 20, len)?
            .chunks_exact(2)
            .map(|x| u16::from_le_bytes([x[0], x[1]]))
            .collect();
        Ok(Self {
            raw: b.to_vec(),
            entry_at: at + 20,
            width,
            height,
            entries,
        })
    }
    pub fn new(width: u16, height: u16, entries: &[u16]) -> Result<Self> {
        let mut body = vec![0; 12];
        put16(&mut body, 0, width);
        put16(&mut body, 2, height);
        put32(&mut body, 8, (entries.len() * 2) as u32);
        for e in entries {
            body.extend_from_slice(&e.to_le_bytes());
        }
        Self::parse(&make(b"RCSN", b"NRCS", &body))
    }
    pub fn write(&self) -> Vec<u8> {
        let mut b = self.raw.clone();
        for (i, e) in self.entries.iter().enumerate() {
            if self.entry_at + i * 2 + 2 <= b.len() {
                put16(&mut b, self.entry_at + i * 2, *e);
            }
        }
        b
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

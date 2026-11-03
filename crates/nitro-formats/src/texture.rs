//! NSBTX reader. Layout reference: apicula (0BSD), src/nitro/{tex,info_block}.rs.
//! Independently implemented with bounded reads and explicit palette selection.
use crate::{
    error,
    graphics::{bgr555_to_rgba, Image},
    slice, u16le, u32le, Result,
};

#[derive(Clone, Debug)]
pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: u8,
    pub transparent_zero: bool,
    pub data: Vec<u8>,
    pub indices: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Palette {
    pub name: String,
    pub colors: Vec<u16>,
}
#[derive(Clone, Debug)]
pub struct Nsbtx {
    raw: Vec<u8>,
    pub textures: Vec<Texture>,
    pub palettes: Vec<Palette>,
}
pub(crate) fn dictionary(b: &[u8], at: usize, stride: usize) -> Result<Vec<(String, Vec<u8>)>> {
    let count = slice(b, at + 1, 1)?[0] as usize;
    let len = u16le(b, at + 2)? as usize;
    let d = slice(b, at, len)?;
    let table = 8 + u16le(d, 6)? as usize;
    if u16le(d, table)? as usize != stride {
        return Err(error(at + table, "unexpected dictionary entry size"));
    }
    let names = table + u16le(d, table + 2)? as usize;
    let mut out = vec![];
    for i in 0..count {
        let name = slice(d, names + i * 16, 16)?;
        let end = name.iter().position(|x| *x == 0).unwrap_or(16);
        out.push((
            String::from_utf8_lossy(&name[..end]).into(),
            slice(d, table + 4 + i * stride, stride)?.to_vec(),
        ));
    }
    Ok(out)
}
impl Nsbtx {
    pub fn parse(b: &[u8]) -> Result<Self> {
        if ![b"BTX0".as_slice(), b"BMD0".as_slice()].contains(&slice(b, 0, 4)?)
            || u16le(b, 4)? != 0xfeff
        {
            return Err(error(0, "expected little-endian BTX0"));
        }
        let size = u32le(b, 8)? as usize;
        if size < 16 || u16le(b, 12)? < 16 {
            return Err(error(8, "invalid NSBTX header size"));
        }
        let b = slice(b, 0, size)?;
        let count = u16le(b, 14)? as usize;
        slice(b, 16, count * 4)?;
        let mut tex = None;
        for i in 0..count {
            let p = u32le(b, 16 + i * 4)? as usize;
            if slice(b, p, 4)? == b"TEX0" {
                let len = u32le(b, p + 4)? as usize;
                tex = Some(slice(b, p, len)?);
            }
        }
        let t = tex.ok_or_else(|| error(16, "missing TEX0"))?;
        let tex_dict = u16le(t, 14)? as usize;
        let pal_dict = u32le(t, 52)? as usize;
        let tex_base = u32le(t, 20)? as usize;
        let comp_base = u32le(t, 36)? as usize;
        let comp_idx = u32le(t, 40)? as usize;
        let tex_len = (u16le(t, 12)? as usize) * 8;
        let comp_len = (u16le(t, 28)? as usize) * 8;
        let normal = slice(t, tex_base, tex_len)?;
        let compressed = slice(t, comp_base, comp_len)?;
        let descriptors = slice(t, comp_idx, comp_len / 2)?;
        let pal_base = u32le(t, 56)? as usize;
        let pal_len = u16le(t, 48)? as usize * 8;
        let pals = slice(t, pal_base, pal_len)?;
        let mut textures = vec![];
        for (name, d) in dictionary(t, tex_dict, 8)? {
            let params = u32le(&d, 0)?;
            let format = ((params >> 26) & 7) as u8;
            if format == 0 {
                return Err(error(tex_dict, "texture has no format"));
            }
            let width = 8 << ((params >> 20) & 7);
            let height = 8 << ((params >> 23) & 7);
            let n = width as usize * height as usize;
            let off = (params as usize & 65535) * 8;
            let len = match format {
                2 | 5 => n / 4,
                3 => n / 2,
                7 => n * 2,
                _ => n,
            };
            let data = slice(if format == 5 { compressed } else { normal }, off, len)?.to_vec();
            let indices = if format == 5 {
                slice(descriptors, off / 2, len / 2)?.to_vec()
            } else {
                vec![]
            };
            textures.push(Texture {
                name,
                width,
                height,
                format,
                transparent_zero: params & (1 << 29) != 0,
                data,
                indices,
            });
        }
        let entries = dictionary(t, pal_dict, 4)?;
        let mut offsets = vec![];
        for (_, d) in &entries {
            let o = u16le(d, 0)? as usize * 8;
            if o > pals.len() {
                return Err(error(pal_base + o, "palette outside data"));
            }
            offsets.push(o);
        }
        let mut palettes = vec![];
        for (i, (name, _)) in entries.into_iter().enumerate() {
            let start = offsets[i];
            let end = offsets
                .iter()
                .copied()
                .filter(|p| *p > start)
                .min()
                .unwrap_or(pals.len());
            let colors = pals[start..end]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            palettes.push(Palette { name, colors });
        }
        Ok(Self {
            raw: b.to_vec(),
            textures,
            palettes,
        })
    }
    pub fn write(&self) -> Vec<u8> {
        self.raw.clone()
    }
    pub fn decode(&self, texture: usize, palette: usize) -> Result<Image> {
        let t = self
            .textures
            .get(texture)
            .ok_or_else(|| error(0, "texture index outside dictionary"))?;
        let colors = self
            .palettes
            .get(palette)
            .map(|p| p.colors.as_slice())
            .unwrap_or(&[]);
        decode(t, colors)
    }
}
pub fn decode(t: &Texture, palette: &[u16]) -> Result<Image> {
    if t.width < 8
        || t.height < 8
        || t.width > 1024
        || t.height > 1024
        || !t.width.is_power_of_two()
        || !t.height.is_power_of_two()
    {
        return Err(error(0, "invalid texture dimensions"));
    }
    let n = t.width as usize * t.height as usize;
    let mut rgba = vec![0; n * 4];
    let color = |idx: usize| -> Result<[u8; 4]> {
        palette
            .get(idx)
            .map(|c| bgr555_to_rgba(*c))
            .ok_or_else(|| error(idx * 2, "palette index outside palette"))
    };
    if t.format == 5 {
        let blocks_x = t.width as usize / 4;
        for block in 0..n / 16 {
            let bits = u32le(&t.data, block * 4)?;
            let desc = u16le(&t.indices, block * 2)?;
            let off = (desc as usize & 0x3fff) * 2;
            let mode = desc >> 14;
            let a = color(off)?;
            let b = color(off + 1)?;
            let mix = |wa: u16, wb: u16| {
                let mut c = [0; 4];
                for k in 0..3 {
                    c[k] = ((a[k] as u16 * wa + b[k] as u16 * wb) / (wa + wb)) as u8;
                }
                c[3] = 255;
                c
            };
            let table = match mode {
                0 => [a, b, color(off + 2)?, [0; 4]],
                1 => [a, b, mix(1, 1), [0; 4]],
                2 => [a, b, color(off + 2)?, color(off + 3)?],
                _ => [a, b, mix(5, 3), mix(3, 5)],
            };
            for j in 0..16 {
                let x = block % blocks_x * 4 + j % 4;
                let y = block / blocks_x * 4 + j / 4;
                let dest = (y * t.width as usize + x) * 4;
                rgba[dest..dest + 4].copy_from_slice(&table[((bits >> (j * 2)) & 3) as usize]);
            }
        }
    } else {
        for i in 0..n {
            let (idx, alpha) = match t.format {
                1 => {
                    let p = slice(&t.data, i, 1)?[0];
                    (p as usize & 31, Some(((p >> 5) as u16 * 255 / 7) as u8))
                }
                2 => (
                    ((slice(&t.data, i / 4, 1)?[0] >> (i % 4 * 2)) & 3) as usize,
                    None,
                ),
                3 => (
                    ((slice(&t.data, i / 2, 1)?[0] >> (i % 2 * 4)) & 15) as usize,
                    None,
                ),
                4 => (slice(&t.data, i, 1)?[0] as usize, None),
                6 => {
                    let p = slice(&t.data, i, 1)?[0];
                    (p as usize & 7, Some(((p >> 3) as u16 * 255 / 31) as u8))
                }
                7 => {
                    let p = u16le(&t.data, i * 2)?;
                    let mut c = bgr555_to_rgba(p);
                    c[3] = if p & 0x8000 != 0 { 255 } else { 0 };
                    rgba[i * 4..i * 4 + 4].copy_from_slice(&c);
                    continue;
                }
                _ => return Err(error(0, "invalid texture format")),
            };
            let mut c = color(idx)?;
            c[3] = alpha.unwrap_or(if t.transparent_zero && idx == 0 {
                0
            } else {
                255
            });
            rgba[i * 4..i * 4 + 4].copy_from_slice(&c);
        }
    }
    Ok(Image {
        width: t.width,
        height: t.height,
        rgba,
    })
}

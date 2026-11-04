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
    if count == 0 {
        return Ok(vec![]);
    }
    // The Patricia tree contains a root plus one node per resource. The
    // subheader fields vary between exporters; entries follow the tree.
    let table = 12 + count * 4;
    if u16le(d, table)? as usize != stride {
        return Err(error(
            at + table,
            format!(
                "unexpected dictionary entry size: {} (expected {stride})",
                u16le(d, table)?
            ),
        ));
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
    /// Construct an authored texture bank; pools and resource trees are built
    /// from scratch, without a retail template.
    pub fn new(textures: Vec<Texture>, palettes: Vec<Palette>) -> Result<Self> {
        use crate::{
            g3d::{align, container, dictionary},
            put16, put32,
        };
        let mut normal = vec![];
        let mut compressed = vec![];
        let mut descriptors = Vec::<u8>::new();
        let mut td = vec![];
        for t in &textures {
            if !(8..=1024).contains(&t.width)
                || !(8..=1024).contains(&t.height)
                || !t.width.is_power_of_two()
                || !t.height.is_power_of_two()
                || !(1..=7).contains(&t.format)
            {
                return Err(error(0, "invalid DS texture dimensions or format"));
            }
            let n = t.width as usize * t.height as usize;
            let expected = match t.format {
                2 | 5 => n / 4,
                3 => n / 2,
                7 => n * 2,
                _ => n,
            };
            if t.data.len() != expected || (t.format == 5 && t.indices.len() != expected / 2) {
                return Err(error(0, "texture pool length mismatch"));
            }
            let pool = if t.format == 5 {
                &mut compressed
            } else {
                &mut normal
            };
            align(pool, 8);
            let offset = pool.len() / 8;
            if offset > 65535 {
                return Err(error(0, "texture pool exceeds hardware range"));
            }
            let param = offset as u32
                | ((t.width.ilog2() - 3) << 20)
                | ((t.height.ilog2() - 3) << 23)
                | ((t.format as u32) << 26)
                | u32::from(t.transparent_zero) << 29;
            let mut d = param.to_le_bytes().to_vec();
            d.extend(((t.width - 1) | ((t.height - 1) << 11)).to_le_bytes());
            td.push((t.name.clone(), d));
            pool.extend(&t.data);
            if t.format == 5 {
                descriptors.extend(&t.indices);
            }
        }
        let mut pd = vec![];
        let mut colors = vec![];
        for p in &palettes {
            align(&mut colors, 8);
            let offset = colors.len() / 8;
            if offset > 65535 {
                return Err(error(0, "palette pool exceeds hardware range"));
            }
            pd.push((p.name.clone(), (offset as u32).to_le_bytes().to_vec()));
            for c in &p.colors {
                colors.extend(c.to_le_bytes());
            }
        }
        let mut tex = vec![0; 60];
        tex[..4].copy_from_slice(b"TEX0");
        let td_at = tex.len();
        tex.extend(dictionary(&td, 8)?);
        let pd_at = tex.len();
        tex.extend(dictionary(&pd, 4)?);
        align(&mut tex, 8);
        put16(&mut tex, 14, td_at as u16);
        put32(&mut tex, 52, pd_at as u32);
        for (pool, size_field, offset_field) in [
            (&mut normal, 12, 20),
            (&mut compressed, 28, 36),
            (&mut colors, 48, 56),
        ] {
            align(pool, 8);
            if pool.len() / 8 > 65535 {
                return Err(error(0, "texture pool exceeds 512 KiB"));
            }
            let at = tex.len();
            put16(&mut tex, size_field, (pool.len() / 8) as u16);
            put32(&mut tex, offset_field, at as u32);
            tex.extend_from_slice(pool);
        }
        let at = tex.len();
        put32(&mut tex, 40, at as u32);
        tex.extend(descriptors);
        let len = tex.len();
        put32(&mut tex, 4, len as u32);
        Self::parse(&container(b"BTX0", &[tex]))
    }
    /// Repack changed pools and relocate the containing blocks. Unrelated dictionary entries,
    /// texture flags, compressed pools and unknown blocks are retained.
    pub fn replace(
        &self,
        texture: usize,
        palette: usize,
        value: &Texture,
        colors: Option<&[u16]>,
    ) -> Result<Vec<u8>> {
        if texture >= self.textures.len() || value.format == 5 {
            return Err(error(
                texture,
                "select a texture; convert 4x4 compressed textures before editing",
            ));
        }
        decode(
            value,
            colors
                .or_else(|| self.palettes.get(palette).map(|p| p.colors.as_slice()))
                .unwrap_or(&[]),
        )?;
        let previous = &self.textures[texture];
        if previous.format == value.format
            && previous.width == value.width
            && previous.height == value.height
            && previous.data.len() == value.data.len()
            && colors.is_none_or(|c| {
                self.palettes
                    .get(palette)
                    .is_some_and(|p| p.colors.len() == c.len())
            })
        {
            let mut out = self.raw.clone();
            for i in 0..u16le(&out, 14)? as usize {
                let at = u32le(&out, 16 + i * 4)? as usize;
                if slice(&out, at, 4)? != b"TEX0" {
                    continue;
                }
                let td = u16le(&out, at + 14)? as usize + at;
                let record = td + 12 + out[td + 1] as usize * 4 + 4 + texture * 8;
                let params = u32le(&out, record)?;
                let dest = at + u32le(&out, at + 20)? as usize + (params as usize & 65535) * 8;
                out[dest..dest + value.data.len()].copy_from_slice(&value.data);
                crate::put32(
                    &mut out,
                    record,
                    (params & !(1 << 29)) | (u32::from(value.transparent_zero) << 29),
                );
                if let Some(c) = colors {
                    let pd = u32le(&out, at + 52)? as usize + at;
                    let record = pd + 12 + out[pd + 1] as usize * 4 + 4 + palette * 4;
                    let dest =
                        at + u32le(&out, at + 56)? as usize + u16le(&out, record)? as usize * 8;
                    for (i, c) in c.iter().enumerate() {
                        crate::put16(&mut out, dest + i * 2, *c);
                    }
                }
            }
            return Ok(out);
        }
        let count = u16le(&self.raw, 14)? as usize;
        let mut blocks = vec![];
        for i in 0..count {
            let at = u32le(&self.raw, 16 + i * 4)? as usize;
            let mut block = slice(&self.raw, at, u32le(&self.raw, at + 4)? as usize)?.to_vec();
            if block.starts_with(b"TEX0") {
                let add_palette = colors.is_some() && self.palettes.is_empty() && palette == 0;
                if colors.is_some() && palette >= self.palettes.len() && !add_palette {
                    return Err(error(palette, "palette index outside dictionary"));
                }
                let td = u16le(&block, 14)? as usize;
                let pd = u32le(&block, 52)? as usize;
                let table = td + 12 + block[td + 1] as usize * 4;
                let mut pal_table = pd + 12 + block[pd + 1] as usize * 4;
                let metadata_end = (td + u16le(&block, td + 2)? as usize)
                    .max(pd + u16le(&block, pd + 2)? as usize)
                    .max(60);
                block.truncate(metadata_end);
                if add_palette {
                    while !block.len().is_multiple_of(4) {
                        block.push(0);
                    }
                    let pd = block.len();
                    let dict = crate::g3d::dictionary(&[(previous.name.clone(), vec![0; 4])], 4)?;
                    block.extend_from_slice(&dict);
                    crate::put32(&mut block, 52, pd as u32);
                    pal_table = pd + 16;
                }
                let mut normal = vec![];
                let mut compressed = vec![];
                let mut descriptors = vec![];
                for (j, old_texture) in self.textures.iter().enumerate() {
                    let t = if j == texture { value } else { old_texture };
                    let pool = if t.format == 5 {
                        &mut compressed
                    } else {
                        &mut normal
                    };
                    let offset = pool.len();
                    pool.extend_from_slice(&t.data);
                    while !pool.len().is_multiple_of(8) {
                        pool.push(0);
                    }
                    if pool.len() / 8 > u16::MAX as usize {
                        return Err(error(at, "texture pool exceeds DS address space"));
                    }
                    if t.format == 5 {
                        descriptors.resize(offset / 2, 0);
                        descriptors.extend_from_slice(&t.indices);
                        descriptors.resize(pool.len() / 2, 0);
                    }
                    let record = table + 4 + j * 8;
                    let old = u32le(&block, record)?;
                    let params = (old & 0xc00f0000)
                        | (offset as u32 / 8)
                        | ((t.width.ilog2() - 3) << 20)
                        | ((t.height.ilog2() - 3) << 23)
                        | ((t.format as u32) << 26)
                        | (u32::from(t.transparent_zero) << 29);
                    crate::put32(&mut block, record, params);
                }
                let mut pals = vec![];
                if add_palette {
                    for color in colors.unwrap_or(&[]) {
                        pals.extend_from_slice(&color.to_le_bytes());
                    }
                    while !pals.len().is_multiple_of(8) {
                        pals.push(0);
                    }
                }
                for (j, p) in self.palettes.iter().enumerate() {
                    let c = if j == palette {
                        colors.unwrap_or(&p.colors)
                    } else {
                        &p.colors
                    };
                    let offset = pals.len();
                    for v in c {
                        pals.extend_from_slice(&v.to_le_bytes());
                    }
                    while !pals.len().is_multiple_of(8) {
                        pals.push(0);
                    }
                    if pals.len() / 8 > u16::MAX as usize {
                        return Err(error(at, "palette pool too large"));
                    }
                    crate::put16(&mut block, pal_table + 4 + j * 4, (offset / 8) as u16);
                }
                for (offset_field, length_field, data) in [
                    (20, Some(12), &normal),
                    (36, Some(28), &compressed),
                    (40, None, &descriptors),
                    (56, Some(48), &pals),
                ] {
                    while !block.len().is_multiple_of(8) {
                        block.push(0);
                    }
                    let start = block.len();
                    block.extend_from_slice(data);
                    crate::put32(&mut block, offset_field, start as u32);
                    if let Some(field) = length_field {
                        crate::put16(&mut block, field, (data.len() / 8) as u16);
                    }
                }
                let len = block.len();
                crate::put32(&mut block, 4, len as u32);
            }
            blocks.push(block);
        }
        let header = u16le(&self.raw, 12)? as usize;
        let mut out = slice(&self.raw, 0, header.max(16 + count * 4))?.to_vec();
        for (i, block) in blocks.iter().enumerate() {
            while !out.len().is_multiple_of(4) {
                out.push(0);
            }
            let at = out.len();
            crate::put32(&mut out, 16 + i * 4, at as u32);
            out.extend_from_slice(block);
        }
        let len = out.len();
        crate::put32(&mut out, 8, len as u32);
        Ok(out)
    }
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
pub fn encode(
    image: &Image,
    format: u8,
    palette: &[u16],
    indices: &[u8],
    transparent_zero: bool,
) -> Result<Texture> {
    let n = image.width as usize * image.height as usize;
    if indices.len() != n || image.rgba.len() != n * 4 || ![1, 2, 3, 4, 6, 7].contains(&format) {
        return Err(error(0, "invalid encoder input"));
    }
    let mut data = vec![];
    match format {
        2 => {
            for chunk in indices.chunks(4) {
                let mut v = 0;
                for (i, p) in chunk.iter().enumerate() {
                    if *p >= 4 {
                        return Err(error(i, "4-colour index out of range"));
                    }
                    v |= *p << (i * 2);
                }
                data.push(v);
            }
        }
        3 => {
            for chunk in indices.chunks(2) {
                if chunk.iter().any(|p| *p >= 16) {
                    return Err(error(0, "16-colour index out of range"));
                }
                data.push(chunk[0] | (chunk.get(1).copied().unwrap_or(0) << 4));
            }
        }
        1 | 6 => {
            let (bits, max) = if format == 1 { (5, 7) } else { (3, 31) };
            for (i, p) in indices.iter().enumerate() {
                if *p >= 1 << bits {
                    return Err(error(i, "alpha-index texture index out of range"));
                }
                let a = (image.rgba[i * 4 + 3] as u16 * max / 255) as u8;
                data.push(*p | a << bits);
            }
        }
        4 => data.extend_from_slice(indices),
        7 => {
            for p in image.rgba.chunks_exact(4) {
                let c = crate::graphics::rgba_to_bgr555([p[0], p[1], p[2], p[3]])
                    | if p[3] >= 128 { 0x8000 } else { 0 };
                data.extend_from_slice(&c.to_le_bytes());
            }
        }
        _ => unreachable!(),
    }
    let t = Texture {
        name: String::new(),
        width: image.width,
        height: image.height,
        format,
        transparent_zero,
        data,
        indices: vec![],
    };
    decode(&t, palette)?;
    Ok(t)
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

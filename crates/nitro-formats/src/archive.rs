use crate::{error, put16, put32, slice, u16le, u32le, Result};
use std::collections::{BTreeMap, BTreeSet};

/// Decode a Nitro filename table, shared by ROMs and archives.
pub fn filenames(b: &[u8], count: usize) -> Result<Vec<Option<String>>> {
    let dirs = u16le(b, 6)? as usize;
    if dirs == 0 || dirs > 4096 {
        return Err(error(6, "invalid directory count"));
    }
    slice(b, 0, dirs * 8)?;
    let mut names = vec![None; count];
    fn walk(
        b: &[u8],
        dir: usize,
        dirs: usize,
        prefix: &str,
        names: &mut [Option<String>],
        seen: &mut BTreeSet<usize>,
        depth: usize,
    ) -> Result<()> {
        if dir >= dirs || depth > 32 || !seen.insert(dir) {
            return Err(error(dir * 8, "invalid or cyclic directory"));
        }
        let mut p = u32le(b, dir * 8)? as usize;
        if p < dirs * 8 {
            return Err(error(p, "directory points into table"));
        }
        let mut id = u16le(b, dir * 8 + 4)? as usize;
        loop {
            let tag = slice(b, p, 1)?[0];
            p += 1;
            if tag == 0 {
                break;
            }
            let len = (tag & 127) as usize;
            if len == 0 {
                return Err(error(p - 1, "empty filename"));
            }
            let name = std::str::from_utf8(slice(b, p, len)?)
                .map_err(|_| error(p, "invalid UTF-8 filename"))?;
            if name == "." || name == ".." || name.contains(['/', '\\', ':']) {
                return Err(error(p, "unsafe filename"));
            }
            p += len;
            if tag & 128 != 0 {
                let child = u16le(b, p)?;
                if child < 0xf000 {
                    return Err(error(p, "invalid child directory id"));
                }
                p += 2;
                walk(
                    b,
                    (child - 0xf000) as usize,
                    dirs,
                    &format!("{prefix}{name}/"),
                    names,
                    seen,
                    depth + 1,
                )?;
            } else {
                let entry = names
                    .get_mut(id)
                    .ok_or_else(|| error(p, "file id outside FAT"))?;
                if entry.is_some() {
                    return Err(error(p, "duplicate file id"));
                }
                *entry = Some(format!("{prefix}{name}"));
                id += 1;
            }
        }
        Ok(())
    }
    walk(b, 0, dirs, "", &mut names, &mut BTreeSet::new(), 0)?;
    let mut unique = BTreeSet::new();
    if names.iter().flatten().any(|s| !unique.insert(s)) {
        return Err(error(0, "duplicate filename"));
    }
    Ok(names)
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub data: Vec<u8>,
    pub offset: usize,
}
#[derive(Clone, Debug)]
pub struct Narc {
    original: Vec<u8>,
    fat_at: usize,
    image_at: usize,
    image_size: usize,
    pub entries: Vec<Entry>,
}

impl Narc {
    pub fn parse(b: &[u8]) -> Result<Self> {
        if slice(b, 0, 4)? != b"NARC" {
            return Err(error(0, "expected NARC"));
        }
        if ![0xfffe, 0xfeff].contains(&u16le(b, 4)?)
            || !(16..=b.len()).contains(&(u32le(b, 8)? as usize))
        {
            return Err(error(4, "invalid NARC header"));
        }
        let mut p = u16le(b, 12)? as usize;
        if p < 16 {
            return Err(error(12, "invalid header size"));
        }
        let mut fat = None;
        let mut fnt = None;
        let mut data = None;
        for _ in 0..u16le(b, 14)? {
            let size = u32le(b, p + 4)? as usize;
            if size < 8
                || p.checked_add(size)
                    .is_none_or(|end| end > u32le(b, 8).unwrap_or(0) as usize)
            {
                return Err(error(p + 4, "invalid block size"));
            }
            let block = slice(b, p, size)?;
            match &block[..4] {
                b"BTAF" if fat.is_none() => fat = Some((p, block)),
                b"BTNF" if fnt.is_none() => fnt = Some((p, &block[8..])),
                b"GMIF" if data.is_none() => data = Some((p + 8, &block[8..])),
                b"BTAF" | b"BTNF" | b"GMIF" => return Err(error(p, "duplicate NARC block")),
                _ => {}
            }
            p += size;
        }
        let (fat_at, fat) = fat.ok_or_else(|| error(16, "missing BTAF"))?;
        let (data_at, data) = data.ok_or_else(|| error(16, "missing GMIF"))?;
        let count = u16le(fat, 8)? as usize;
        slice(fat, 12, count * 8)?;
        let names = match fnt {
            // SDK unnamed FNT is a single root record with no name list.
            Some((_, f)) if f.len() == 8 && u32le(f, 0)? == 4 => vec![None; count],
            Some((at, f)) => {
                filenames(f, count).map_err(|e| error(at + 8 + e.offset, e.message))?
            }
            None => return Err(error(16, "missing BTNF")),
        };
        let mut entries = Vec::new();
        for (i, name) in names.into_iter().enumerate() {
            let start = u32le(fat, 12 + i * 8)? as usize;
            let end = u32le(fat, 16 + i * 8)? as usize;
            let len = end
                .checked_sub(start)
                .ok_or_else(|| error(fat_at + 12 + i * 8, "reversed file range"))?;
            let bytes = slice(data, start, len)?.to_vec();
            entries.push(Entry {
                name: name.unwrap_or_else(|| format!("#{i}")),
                data: bytes,
                offset: data_at + start,
            });
        }
        Ok(Self {
            original: b.to_vec(),
            fat_at,
            image_at: data_at - 8,
            image_size: data.len() + 8,
            entries,
        })
    }
    /// Keep every byte, including nonstandard padding, for an unmodified archive.
    pub fn write(&self) -> Vec<u8> {
        let mut result = self.original.clone();
        if let Ok(old) = Self::parse(&self.original) {
            if old.entries.len() == self.entries.len()
                && old
                    .entries
                    .iter()
                    .zip(&self.entries)
                    .all(|(a, b)| a.data.len() == b.data.len())
            {
                for (a, b) in old.entries.iter().zip(&self.entries) {
                    result[a.offset..a.offset + b.data.len()].copy_from_slice(&b.data);
                }
                return result;
            }
        }
        // Rebuild only GMIF and FAT ranges. Keep BTNF, unknown blocks, block order,
        // header extensions and trailing bytes, even when GMIF is not the final block.
        let old = Self::parse(&self.original).expect("Narc retains its validated original");
        if old.entries.len() != self.entries.len() {
            return Self::new(
                self.entries
                    .iter()
                    .map(|e| (e.name.clone(), e.data.clone()))
                    .collect(),
            )
            .map(|a| a.original)
            .unwrap_or(result);
        }
        let data_at = self.image_at + 8;
        let aligned = old
            .entries
            .iter()
            .all(|e| (e.offset - data_at).is_multiple_of(4));
        let padding = old
            .entries
            .windows(2)
            .find_map(|pair| {
                let end = pair[0].offset + pair[0].data.len();
                (end < pair[1].offset).then(|| self.original[end])
            })
            .unwrap_or(0xff);
        let mut payload = vec![];
        let mut ranges = vec![];
        for entry in &self.entries {
            if aligned {
                while !payload.len().is_multiple_of(4) {
                    payload.push(padding);
                }
            }
            let start = payload.len();
            payload.extend_from_slice(&entry.data);
            ranges.push((start, payload.len()));
        }
        while !payload.len().is_multiple_of(4) {
            payload.push(padding);
        }
        for (i, (start, end)) in ranges.into_iter().enumerate() {
            put32(&mut result, self.fat_at + 12 + i * 8, start as u32);
            put32(&mut result, self.fat_at + 16 + i * 8, end as u32);
        }
        let new_image_size = payload.len() + 8;
        put32(&mut result, self.image_at + 4, new_image_size as u32);
        result.splice(data_at..self.image_at + self.image_size, payload);
        // The declared length excludes any bytes trailing the declared file.
        let declared = u32le(&self.original, 8).unwrap() as usize;
        put32(
            &mut result,
            8,
            (declared - self.image_size + new_image_size) as u32,
        );
        result
    }
    /// Construct an archive from author-created files. Directories are encoded in BTNF.
    pub fn new(files: BTreeMap<String, Vec<u8>>) -> Result<Self> {
        #[derive(Default)]
        struct Dir {
            files: Vec<(String, Vec<u8>)>,
            children: BTreeMap<String, Dir>,
        }
        let mut root = Dir::default();
        for (path, bytes) in files {
            let parts: Vec<_> = path.split('/').collect();
            let mut dir = &mut root;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty()
                    || part.len() > 127
                    || matches!(*part, "." | "..")
                    || part.contains(['\\', ':'])
                {
                    return Err(error(0, "invalid archive path"));
                }
                if i + 1 == parts.len() {
                    dir.files.push(((*part).into(), bytes.clone()));
                } else {
                    dir = dir.children.entry((*part).into()).or_default();
                }
            }
        }
        struct Record {
            parent: usize,
            children: Vec<(String, usize)>,
            files: Vec<(String, Vec<u8>)>,
        }
        fn flatten(d: Dir, parent: usize, records: &mut Vec<Record>) -> usize {
            let id = records.len();
            records.push(Record {
                parent,
                children: vec![],
                files: d.files,
            });
            for (name, child) in d.children {
                let sub = flatten(child, id, records);
                records[id].children.push((name, sub));
            }
            id
        }
        let mut records = vec![];
        flatten(root, 0, &mut records);
        if records.len() > 4096 {
            return Err(error(0, "too many directories"));
        }
        let count: usize = records.iter().map(|d| d.files.len()).sum();
        if count > 65535 {
            return Err(error(0, "too many files"));
        }
        let mut fnt = vec![0; records.len() * 8];
        let mut payload = vec![];
        let mut ranges = vec![];
        let mut id = 0;
        for (i, r) in records.iter().enumerate() {
            let off = fnt.len() as u32;
            put32(&mut fnt, i * 8, off);
            put16(&mut fnt, i * 8 + 4, id);
            put16(
                &mut fnt,
                i * 8 + 6,
                if i == 0 {
                    records.len() as u16
                } else {
                    0xf000 + r.parent as u16
                },
            );
            for (name, bytes) in &r.files {
                fnt.push(name.len() as u8);
                fnt.extend_from_slice(name.as_bytes());
                while payload.len() % 4 != 0 {
                    payload.push(0xff);
                }
                let start = payload.len() as u32;
                payload.extend_from_slice(bytes);
                ranges.push((start, payload.len() as u32));
                id += 1;
            }
            for (name, child) in &r.children {
                fnt.push(128 | name.len() as u8);
                fnt.extend_from_slice(name.as_bytes());
                fnt.extend_from_slice(&(0xf000 + *child as u16).to_le_bytes());
            }
            fnt.push(0);
        }
        while !fnt.len().is_multiple_of(4) {
            fnt.push(0xff);
        }
        while payload.len() % 4 != 0 {
            payload.push(0xff);
        }
        let mut out = b"NARC\xfe\xff\x00\x01\0\0\0\0\x10\0\x03\0".to_vec();
        out.extend_from_slice(b"BTAF");
        out.extend_from_slice(&((12 + count * 8) as u32).to_le_bytes());
        out.extend_from_slice(&(count as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        for (a, b) in ranges {
            out.extend_from_slice(&a.to_le_bytes());
            out.extend_from_slice(&b.to_le_bytes());
        }
        for (tag, bytes) in [(b"BTNF", fnt), (b"GMIF", payload)] {
            out.extend_from_slice(tag);
            out.extend_from_slice(&((8 + bytes.len()) as u32).to_le_bytes());
            out.extend_from_slice(&bytes);
        }
        let len = out.len() as u32;
        put32(&mut out, 8, len);
        Self::parse(&out)
    }
}

use super::{header, Sbnk, Ssar, Sseq, Strm, Swar};
use crate::{error, g3d::align, put16, put32, slice, u16le, u32le, Result};
#[derive(Clone, Debug)]
pub struct Sdat {
    pub files: Vec<Vec<u8>>,
    pub info: [Vec<Option<Vec<u8>>>; 8],
    pub symbols: [Vec<Option<String>>; 8],
    raw: Vec<u8>,
    original_files: Vec<Vec<u8>>,
    original_info: [Vec<Option<Vec<u8>>>; 8],
}
impl Default for Sdat {
    fn default() -> Self {
        Self {
            files: vec![],
            info: std::array::from_fn(|_| vec![]),
            symbols: std::array::from_fn(|_| vec![]),
            raw: vec![],
            original_files: vec![],
            original_info: std::array::from_fn(|_| vec![]),
        }
    }
}
impl Sdat {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let _ = header(b, b"SDAT")?;
        slice(b, 0, 64)?;
        let mut out = Self {
            raw: b.to_vec(),
            ..Self::default()
        };
        let mut blocks = vec![];
        for i in 0..4 {
            let at = u32le(b, 16 + i * 8)? as usize;
            let len = u32le(b, 20 + i * 8)? as usize;
            if at == 0 && i == 0 {
                blocks.push(&b[0..0]);
                continue;
            }
            let block = slice(b, at, len)?;
            let expected = [b"SYMB", b"INFO", b"FAT ", b"FILE"][i];
            if slice(block, 0, 4)? != expected {
                return Err(error(at, "unexpected SDAT block"));
            }
            blocks.push(block);
        }
        let fat = blocks[2];
        let count = u32le(fat, 8)? as usize;
        if count > 65535 {
            return Err(error(8, "too many SDAT files"));
        }
        slice(fat, 12, count * 16)?;
        for i in 0..count {
            let p = 12 + i * 16;
            let at = u32le(fat, p)? as usize;
            let len = u32le(fat, p + 4)? as usize;
            let file_at = u32le(b, 40)? as usize;
            if at < file_at + 12
                || at
                    .checked_add(len)
                    .is_none_or(|end| end > file_at + blocks[3].len())
            {
                return Err(error(p, "SDAT file outside FILE block"));
            }
            out.files.push(slice(b, at, len)?.to_vec());
        }
        for kind in 0..8 {
            let info = blocks[1];
            let table = u32le(info, 8 + kind * 4)? as usize;
            if table != 0 {
                let n = u32le(info, table)? as usize;
                if n > 65535 {
                    return Err(error(table, "SDAT INFO count exceeds limit"));
                }
                slice(info, table + 4, n * 4)?;
                for i in 0..n {
                    let at = u32le(info, table + 4 + i * 4)? as usize;
                    if at == 0 {
                        out.info[kind].push(None);
                        continue;
                    }
                    let len = match kind {
                        0 => 10,
                        1 | 3 => 4,
                        2 => 12,
                        4 => 8,
                        5 => {
                            4 + (u32le(info, at)? as usize)
                                .checked_mul(8)
                                .ok_or_else(|| error(at, "group size overflow"))?
                        }
                        6 => 1 + slice(info, at, 1)?[0] as usize,
                        _ => 8,
                    };
                    let record = slice(info, at, len)?.to_vec();
                    if [0, 1, 2, 3, 7].contains(&kind)
                        && u16le(&record, 0)? as usize >= out.files.len()
                    {
                        return Err(error(at, "INFO file id outside FAT"));
                    }
                    out.info[kind].push(Some(record));
                }
            }
            if !blocks[0].is_empty() {
                let sym = blocks[0];
                let table = u32le(sym, 8 + kind * 4)? as usize;
                if table != 0 {
                    let n = u32le(sym, table)? as usize;
                    if n > 65535 {
                        return Err(error(table, "SYMB count exceeds limit"));
                    }
                    let stride = if kind == 1 { 8 } else { 4 };
                    for i in 0..n {
                        let at = u32le(sym, table + 4 + i * stride)? as usize;
                        let value = if at == 0 {
                            None
                        } else {
                            let remaining = sym
                                .get(at..)
                                .ok_or_else(|| error(at, "SYMB string outside block"))?;
                            let end = remaining
                                .iter()
                                .position(|&c| c == 0)
                                .ok_or_else(|| error(at, "unterminated SYMB name"))?;
                            Some(remaining[..end].iter().map(|&c| char::from(c)).collect())
                        };
                        out.symbols[kind].push(value);
                    }
                }
            }
        }
        out.original_files = out.files.clone();
        out.original_info = out.info.clone();
        Ok(out)
    }
    pub fn resource_file(&self, kind: usize, index: usize) -> Result<usize> {
        let record = self
            .info
            .get(kind)
            .and_then(|v| v.get(index))
            .and_then(Option::as_ref)
            .ok_or_else(|| error(index, "missing SDAT resource"))?;
        let id = u16le(record, 0)? as usize;
        if id >= self.files.len() {
            return Err(error(id, "SDAT file id outside FAT"));
        }
        Ok(id)
    }
    pub fn bank_for_sequence(&self, index: usize) -> Result<usize> {
        let r = self.info[0]
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| error(index, "missing sequence"))?;
        Ok(u16le(r, 4)? as usize)
    }
    pub fn waves_for_bank(&self, index: usize) -> Result<[u16; 4]> {
        let r = self.info[2]
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| error(index, "missing bank"))?;
        Ok([u16le(r, 4)?, u16le(r, 6)?, u16le(r, 8)?, u16le(r, 10)?])
    }
    pub fn validate_files(&self) -> Result<()> {
        for b in &self.files {
            if b.starts_with(b"SSEQ") {
                Sseq::parse(b)?;
            } else if b.starts_with(b"SBNK") {
                Sbnk::parse(b)?;
            } else if b.starts_with(b"SWAR") {
                Swar::parse(b)?;
            } else if b.starts_with(b"SSAR") {
                Ssar::parse(b)?;
            } else if b.starts_with(b"STRM") {
                Strm::parse(b)?;
            } else {
                return Err(error(0, "unsupported SDAT child format"));
            }
        }
        Ok(())
    }
    pub fn write(&self) -> Result<Vec<u8>> {
        if !self.raw.is_empty()
            && self.files == self.original_files
            && self.info == self.original_info
        {
            return Ok(self.raw.clone());
        }
        if self.files.len() > 65535 {
            return Err(error(0, "too many SDAT files"));
        }
        let mut b = vec![0; 64];
        b[..4].copy_from_slice(b"SDAT");
        put16(&mut b, 4, 0xfeff);
        put16(&mut b, 6, 0x100);
        put16(&mut b, 12, 64);
        put16(&mut b, 14, 4);
        // Retain nested SSAR symbols and unknown metadata from an existing
        // archive verbatim. Imported resources receive numeric names.
        let mut sym = if !self.raw.is_empty() {
            let at = u32le(&self.raw, 16)? as usize;
            let len = u32le(&self.raw, 20)? as usize;
            if at > 0 {
                slice(&self.raw, at, len)?.to_vec()
            } else {
                vec![]
            }
        } else {
            vec![]
        };
        if sym.is_empty() {
            sym = vec![0; 64];
            sym[..4].copy_from_slice(b"SYMB");
            for kind in 0..8 {
                let table = sym.len();
                put32(&mut sym, 8 + kind * 4, table as u32);
                sym.extend((self.info[kind].len() as u32).to_le_bytes());
                let stride = if kind == 1 { 8 } else { 4 };
                sym.resize(sym.len() + self.info[kind].len() * stride, 0);
                for (i, name) in self.symbols[kind]
                    .iter()
                    .take(self.info[kind].len())
                    .enumerate()
                {
                    if let Some(name) = name {
                        let at = sym.len();
                        put32(&mut sym, table + 4 + i * stride, at as u32);
                        sym.extend(name.bytes().filter(|c| *c != 0 && c.is_ascii()));
                        sym.push(0);
                    }
                }
            }
            align(&mut sym, 4);
            let len = sym.len();
            put32(&mut sym, 4, len as u32);
        }
        let at = b.len();
        put32(&mut b, 16, at as u32);
        put32(&mut b, 20, sym.len() as u32);
        b.extend(sym);
        align(&mut b, 32);
        let mut info = vec![0; 64];
        info[..4].copy_from_slice(b"INFO");
        for kind in 0..8 {
            let table = info.len();
            put32(&mut info, 8 + kind * 4, table as u32);
            info.extend((self.info[kind].len() as u32).to_le_bytes());
            info.resize(info.len() + self.info[kind].len() * 4, 0);
            for (i, record) in self.info[kind].iter().enumerate() {
                if let Some(record) = record {
                    align(&mut info, 4);
                    let at = info.len();
                    put32(&mut info, table + 4 + i * 4, at as u32);
                    info.extend(record);
                }
            }
            align(&mut info, 4);
        }
        let len = info.len();
        put32(&mut info, 4, len as u32);
        let at = b.len();
        put32(&mut b, 24, at as u32);
        put32(&mut b, 28, len as u32);
        b.extend(info);
        align(&mut b, 32);
        let fat_at = b.len();
        let fat_len = 12 + self.files.len() * 16;
        put32(&mut b, 32, fat_at as u32);
        put32(&mut b, 36, fat_len as u32);
        b.resize(fat_at + fat_len, 0);
        b[fat_at..fat_at + 4].copy_from_slice(b"FAT ");
        put32(&mut b, fat_at + 4, fat_len as u32);
        put32(&mut b, fat_at + 8, self.files.len() as u32);
        align(&mut b, 32);
        let file_at = b.len();
        put32(&mut b, 40, file_at as u32);
        b.extend(b"FILE\0\0\0\0");
        b.extend((self.files.len() as u32).to_le_bytes());
        for (i, file) in self.files.iter().enumerate() {
            align(&mut b, 32);
            let at = b.len();
            put32(&mut b, fat_at + 12 + i * 16, at as u32);
            put32(&mut b, fat_at + 16 + i * 16, file.len() as u32);
            b.extend(file);
        }
        align(&mut b, 32);
        let len = b.len();
        put32(&mut b, file_at + 4, (len - file_at) as u32);
        put32(&mut b, 44, (len - file_at) as u32);
        put32(&mut b, 8, len as u32);
        Ok(b)
    }
    /// Append an independent sampled bank/wave archive; only the chosen
    /// sequence's bank reference changes, so other songs retain their bank.
    pub fn replace_sampled(
        &mut self,
        sequence: usize,
        seq: Sseq,
        bank: Sbnk,
        waves: Swar,
    ) -> Result<()> {
        let file = self.resource_file(0, sequence)?;
        let bank_id = self.info[2].len();
        let wave_id = self.info[3].len();
        let bank_file = self.files.len();
        if bank_id > 65534 || wave_id > 65534 || bank_file > 65533 {
            return Err(error(0, "SDAT resource id limit"));
        }
        self.files[file] = seq.write();
        self.files.push(bank.write()?);
        self.files.push(waves.write()?);
        let mut br = vec![0; 12];
        put16(&mut br, 0, bank_file as u16);
        for k in 0..4 {
            put16(
                &mut br,
                4 + k * 2,
                if k == 0 { wave_id as u16 } else { 65535 },
            );
        }
        self.info[2].push(Some(br));
        let mut wr = vec![0; 4];
        put16(&mut wr, 0, (bank_file + 1) as u16);
        self.info[3].push(Some(wr));
        put16(self.info[0][sequence].as_mut().unwrap(), 4, bank_id as u16);
        Ok(())
    }
}

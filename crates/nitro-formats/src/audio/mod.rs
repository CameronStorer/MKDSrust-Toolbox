//! Original bounded Nitro sound encoders/readers. Format references: ndspy
//! (RoadrunnerWMC, GPL-3.0-or-later) and public Nitro format documentation.
//! No Nintendo SDK or game code is used.
mod bank;
mod sdat;
mod sequence;
mod stream;
mod wave;
use crate::{error, put16, put32, slice, u16le, u32le, Result};
pub use bank::*;
pub use sdat::*;
pub use sequence::*;
pub use stream::*;
pub use wave::*;
fn header<'a>(b: &'a [u8], magic: &[u8; 4]) -> Result<&'a [u8]> {
    if slice(b, 0, 4)? != magic
        || u16le(b, 4)? != 0xfeff
        || u16le(b, 6)? != 0x100
        || u16le(b, 12)? < 16
    {
        return Err(error(0, "invalid Nitro audio header"));
    }
    let len = u32le(b, 8)? as usize;
    if len < 24 {
        return Err(error(8, "invalid Nitro audio size"));
    }
    slice(b, 0, len)
}
fn file(magic: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut b = vec![0; 24];
    b[..4].copy_from_slice(magic);
    put16(&mut b, 4, 0xfeff);
    put16(&mut b, 6, 0x100);
    put32(&mut b, 8, (24 + body.len()) as u32);
    put16(&mut b, 12, 16);
    put16(&mut b, 14, 1);
    b[16..20].copy_from_slice(b"DATA");
    put32(&mut b, 20, (8 + body.len()) as u32);
    b.extend(body);
    b
}
fn data<'a>(b: &'a [u8], magic: &[u8; 4]) -> Result<&'a [u8]> {
    let b = header(b, magic)?;
    if slice(b, 16, 4)? != b"DATA" {
        return Err(error(16, "missing audio DATA"));
    }
    let n = u32le(b, 20)? as usize;
    if n < 8 {
        return Err(error(20, "invalid DATA size"));
    }
    slice(b, 24, n - 8)
}

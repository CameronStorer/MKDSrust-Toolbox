//! Nitro animation containers and per-frame joint TRS. Layout reference: apicula (0BSD).
use crate::{
    error, math,
    model::{blocks, pivot},
    nkm::fx,
    slice,
    texture::dictionary,
    u16le, u32le, Result,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JointFrames {
    pub bone: usize,
    pub translations: Option<Vec<[f32; 3]>>,
    pub rotations: Option<Vec<[f32; 4]>>,
    pub scales: Option<Vec<[f32; 3]>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animation {
    pub name: String,
    pub frames: u16,
    pub joints: Vec<JointFrames>,
}
#[derive(Clone, Debug)]
pub struct AnimationHeader {
    pub name: String,
    pub frames: u16,
    pub kind: String,
}
pub fn list(b: &[u8]) -> Result<Vec<AnimationHeader>> {
    let magic: [u8; 4] = slice(b, 0, 4)?
        .try_into()
        .map_err(|_| error(0, "animation magic"))?;
    let block = match &magic {
        b"BCA0" => b"JNT0",
        b"BTA0" => b"SRT0",
        b"BTP0" => b"PAT0",
        b"BMA0" => b"MAT0",
        _ => return Err(error(0, "unsupported animation container")),
    };
    let sections = blocks(b, &magic)?;
    let s = sections
        .iter()
        .find(|s| s.starts_with(block))
        .ok_or_else(|| error(16, "missing animation section"))?;
    dictionary(s, 8, 4)?
        .into_iter()
        .map(|(name, d)| {
            let at = u32le(&d, 0)? as usize;
            Ok(AnimationHeader {
                name,
                frames: u16le(s, at + 4)?,
                kind: String::from_utf8_lossy(block).into(),
            })
        })
        .collect()
}
pub fn parse(b: &[u8]) -> Result<Vec<Animation>> {
    let sections = blocks(b, b"BCA0")?;
    let s = sections
        .iter()
        .find(|s| s.starts_with(b"JNT0"))
        .ok_or_else(|| error(16, "missing JNT0"))?;
    dictionary(s, 8, 4)?
        .into_iter()
        .map(|(name, d)| {
            let at = u32le(&d, 0)? as usize;
            decode(slice(s, at, s.len().saturating_sub(at))?, name)
        })
        .collect()
}
fn f16(b: &[u8], at: usize) -> Result<f32> {
    Ok(u16le(b, at)? as i16 as f32 / 4096.)
}
struct Curve {
    start: usize,
    end: usize,
    step: usize,
    width: usize,
    offset: usize,
}
fn curve(b: &[u8], at: &mut usize) -> Result<Curve> {
    let v = u32le(b, *at)?;
    let offset = u32le(b, *at + 4)? as usize;
    *at += 8;
    let start = (v & 65535) as usize;
    let end = ((v >> 16) & 4095) as usize;
    if start >= end {
        return Err(error(*at - 8, "invalid animation curve interval"));
    }
    Ok(Curve {
        start,
        end,
        step: 1 << (v >> 30),
        width: if (v >> 28) & 3 == 0 { 4 } else { 2 },
        offset,
    })
}
fn sampled<T: Clone>(
    c: &Curve,
    frames: usize,
    values: &[T],
    mix: impl Fn(&T, &T, f32) -> T,
) -> Vec<T> {
    (0..frames)
        .map(|f| {
            let x =
                (f.saturating_sub(c.start) as f32 / c.step as f32).min((values.len() - 1) as f32);
            let i = x.floor() as usize;
            mix(
                &values[i],
                &values[(i + 1).min(values.len() - 1)],
                x - i as f32,
            )
        })
        .collect()
}
fn scalar(
    b: &[u8],
    at: &mut usize,
    constant: bool,
    scale: bool,
    frames: usize,
) -> Result<Vec<f32>> {
    if constant {
        let v = fx(b, *at)?;
        *at += if scale { 8 } else { 4 };
        return Ok(vec![v; frames]);
    }
    let c = curve(b, at)?;
    let count = (c.end - c.start).div_ceil(c.step);
    let mut values = vec![];
    for i in 0..count {
        let p = c.offset + i * c.width * if scale { 2 } else { 1 };
        values.push(if c.width == 4 { fx(b, p)? } else { f16(b, p)? })
    }
    Ok(sampled(&c, frames, &values, |a, b, t| a * (1. - t) + b * t))
}
pub fn slerp(a: [f32; 4], mut b: [f32; 4], t: f32) -> [f32; 4] {
    let mut d: f32 = a.iter().zip(b).map(|(a, b)| a * b).sum();
    if d < 0. {
        b = b.map(|v| -v);
        d = -d
    }
    let q = if d > 0.9995 {
        std::array::from_fn(|i| a[i] * (1. - t) + b[i] * t)
    } else {
        let theta = d.clamp(-1., 1.).acos();
        let s = theta.sin();
        std::array::from_fn(|i| (a[i] * ((1. - t) * theta).sin() + b[i] * (t * theta).sin()) / s)
    };
    let n = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    q.map(|v| v / n.max(f32::EPSILON))
}
fn decode(b: &[u8], name: String) -> Result<Animation> {
    if slice(b, 0, 4)? != b"J\0AC" {
        return Err(error(0, "invalid joint animation tag"));
    }
    let frames = u16le(b, 4)?;
    let count = u16le(b, 6)? as usize;
    if frames == 0 || frames as usize * count > 2_000_000 {
        return Err(error(4, "animation frame budget exceeded"));
    }
    slice(b, 20, count * 2)?;
    let pivot_at = u32le(b, 12)? as usize;
    let basis_at = u32le(b, 16)? as usize;
    let rotation = |v: u16| -> Result<[f32; 4]> {
        let i = (v & 0x7fff) as usize;
        let m = if v & 0x8000 != 0 {
            let at = pivot_at + i * 6;
            let flags = u16le(b, at)?;
            pivot(flags & 15, flags >> 4, f16(b, at + 2)?, f16(b, at + 4)?)?
        } else {
            let at = basis_at + i * 10;
            let input = [
                u16le(b, at + 8)?,
                u16le(b, at)?,
                u16le(b, at + 2)?,
                u16le(b, at + 4)?,
                u16le(b, at + 6)?,
            ];
            let mut out = [0u16; 6];
            for j in 0..5 {
                out[j] = input[j] >> 3;
                out[5] = (out[5] << 3) | (input[j] & 7)
            }
            let f = |v: u16| ((v << 3) as i16 >> 3) as f32 / 4096.;
            let a = [f(out[1]), f(out[2]), f(out[3])];
            let c = [f(out[4]), f(out[0]), f(out[5])];
            let d = math::cross(a, c);
            [a[0], a[1], a[2], c[0], c[1], c[2], d[0], d[1], d[2]]
        };
        Ok(math::quaternion(m))
    };
    let mut joints = vec![];
    for j in 0..count {
        let mut at = u16le(b, 20 + j * 2)? as usize;
        let flags = u16le(b, at)?;
        let bone = slice(b, at + 3, 1)?[0] as usize;
        at += 4;
        if bone >= count {
            return Err(error(at - 1, "animation joint index out of range"));
        }
        let mut joint = JointFrames {
            bone,
            translations: None,
            rotations: None,
            scales: None,
        };
        if flags & 1 == 0 {
            if flags & 6 == 0 {
                let mut axes = vec![];
                for i in 0..3 {
                    axes.push(scalar(
                        b,
                        &mut at,
                        flags & (1 << (3 + i)) != 0,
                        false,
                        frames as usize,
                    )?)
                }
                joint.translations = Some(
                    (0..frames as usize)
                        .map(|i| [axes[0][i], axes[1][i], axes[2][i]])
                        .collect(),
                )
            }
            if flags & 0xc0 == 0 {
                joint.rotations = Some(if flags & 0x100 != 0 {
                    let q = rotation(u16le(b, at)?)?;
                    at += 4;
                    vec![q; frames as usize]
                } else {
                    let c = curve(b, &mut at)?;
                    let n = (c.end - c.start).div_ceil(c.step);
                    let values = (0..n)
                        .map(|i| rotation(u16le(b, c.offset + i * 2)?))
                        .collect::<Result<Vec<_>>>()?;
                    sampled(&c, frames as usize, &values, |a, b, t| slerp(*a, *b, t))
                })
            }
            if flags & 0x600 == 0 {
                let mut axes = vec![];
                for i in 0..3 {
                    axes.push(scalar(
                        b,
                        &mut at,
                        flags & (1 << (11 + i)) != 0,
                        true,
                        frames as usize,
                    )?)
                }
                joint.scales = Some(
                    (0..frames as usize)
                        .map(|i| [axes[0][i], axes[1][i], axes[2][i]])
                        .collect(),
                )
            }
        }
        joints.push(joint);
    }
    Ok(Animation {
        name,
        frames,
        joints,
    })
}

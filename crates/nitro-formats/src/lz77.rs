// SPDX-License-Identifier: GPL-3.0-or-later
// Ported from CameronStorer/mkds-native-builder, 1_build_tool/src/dsfiles.rs.
//! DS file formats a mod package may contain (PLUGIN_FORMAT.md "Allowed file types"), mirroring the site's
//! `src/lib/filetypes.ts` rule for rule: known extension, magic matches the extension, `.carc` is LZ77 (type 0x10)
//! compressed NARC, `.kcl` header offsets are sane, and every file inside a NARC/CARC passes the same rules.
//! Also the archive helpers the mod workspace needs: LZ77 compression and NARC extraction / rebuilding.

use crate::{error, slice, u32le, Result};
pub fn decompress(b: &[u8]) -> Result<Vec<u8>> {
    decompress_bounded(b, 64 * 1024 * 1024)
}
fn decompress_bounded(b: &[u8], max_out: usize) -> Result<Vec<u8>> {
    if slice(b, 0, 1)?[0] != 0x10 {
        return Err(error(0, "expected LZ77 type 0x10"));
    }
    let header = slice(b, 0, 4)?;
    let mut size = header[1] as usize | (header[2] as usize) << 8 | (header[3] as usize) << 16;
    let mut p = 4;
    if size == 0 {
        size = u32le(b, 4)? as usize;
        p = 8;
    }
    if size > max_out {
        return Err(error(1, "decompressed size exceeds limit"));
    }
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        let flags = slice(b, p, 1)?[0];
        p += 1;
        for bit in (0..8).rev() {
            if out.len() == size {
                break;
            }
            if flags & (1 << bit) == 0 {
                out.push(slice(b, p, 1)?[0]);
                p += 1;
            } else {
                let pair = slice(b, p, 2)?;
                let len = (pair[0] >> 4) as usize + 3;
                let distance = (((pair[0] & 15) as usize) << 8 | pair[1] as usize) + 1;
                if distance > out.len() {
                    return Err(error(p, "back-reference before start of output"));
                }
                p += 2;
                for _ in 0..len.min(size - out.len()) {
                    out.push(out[out.len() - distance]);
                }
            }
        }
    }
    Ok(out)
}

/// Compress to LZ77 type 0x10 (greedy, hash chains over the 4 KiB window). Output always decompresses to `data`.
pub fn compress(data: &[u8]) -> Vec<u8> {
    let n = data.len();
    let mut out = Vec::with_capacity(n / 2 + 16);
    if n > 0 && n < 1 << 24 {
        out.extend_from_slice(&[0x10, n as u8, (n >> 8) as u8, (n >> 16) as u8]);
    } else {
        out.extend_from_slice(&[0x10, 0, 0, 0]);
        out.extend_from_slice(&(n as u32).to_le_bytes());
    }
    const NONE: usize = usize::MAX;
    let mut head = vec![NONE; 1 << 16];
    let mut prev = vec![NONE; n];
    let key = |i: usize| {
        ((data[i] as usize) << 8 ^ (data[i + 1] as usize) << 4 ^ data[i + 2] as usize) & 0xffff
    };
    let insert = |i: usize, head: &mut Vec<usize>, prev: &mut Vec<usize>| {
        if i + 2 < n {
            let k = key(i);
            prev[i] = head[k];
            head[k] = i;
        }
    };
    let mut i = 0;
    while i < n {
        let flag_at = out.len();
        out.push(0);
        let mut flags = 0u8;
        for bit in (0..8).rev() {
            if i >= n {
                break;
            }
            let (mut best_len, mut best_disp) = (0, 0);
            if i + 2 < n {
                let mut cand = head[key(i)];
                let mut tries = 0;
                while cand != NONE && i - cand <= 0x1000 && tries < 256 {
                    if i - cand < 2 {
                        cand = prev[cand];
                        tries += 1;
                        continue;
                    }
                    let disp = i - cand;
                    // disp 1 is legal for this format (VRAM-safe variants avoid it; the game's decoder accepts it)
                    let max = 18.min(n - i);
                    let mut l = 0;
                    while l < max && data[cand + l] == data[i + l] {
                        l += 1;
                    }
                    if l > best_len {
                        best_len = l;
                        best_disp = disp;
                        if l == max {
                            break;
                        }
                    }
                    cand = prev[cand];
                    tries += 1;
                }
            }
            if best_len >= 3 {
                flags |= 1 << bit;
                let d = best_disp - 1;
                out.push(((best_len - 3) as u8) << 4 | (d >> 8) as u8);
                out.push(d as u8);
                for j in i..i + best_len {
                    insert(j, &mut head, &mut prev);
                }
                i += best_len;
            } else {
                out.push(data[i]);
                insert(i, &mut head, &mut prev);
                i += 1;
            }
        }
        out[flag_at] = flags;
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}

// ---------------------------------------------------------------- minimal-change recompression

#[derive(Clone, Copy, PartialEq, Debug)]
struct Tok {
    len: usize,
    /// 0 = literal
    disp: usize,
}

/// A parsed LZ77 type 0x10 stream: tokens with their output positions, the byte offset of each 8-token group's
/// flag byte, the header length and the offset just past the last token.
struct Lz77Stream {
    toks: Vec<(usize, Tok)>,
    groups: Vec<usize>,
    header: usize,
    end: usize,
}

fn lz77_stream(b: &[u8]) -> Option<Lz77Stream> {
    if b.len() < 4 || b[0] != 0x10 {
        return None;
    }
    let mut size = b[1] as usize | (b[2] as usize) << 8 | (b[3] as usize) << 16;
    let mut p = 4;
    if size == 0 {
        size = u32le(b, 4).ok()? as usize;
        p = 8;
    }
    let header = p;
    let mut out = 0;
    let (mut toks, mut groups) = (Vec::new(), Vec::new());
    while out < size {
        groups.push(p);
        let flags = *b.get(p)?;
        p += 1;
        for bit in (0..8).rev() {
            if out >= size {
                break;
            }
            if flags & (1 << bit) != 0 {
                let (x, y) = (*b.get(p)?, *b.get(p + 1)?);
                p += 2;
                let t = Tok {
                    len: (x >> 4) as usize + 3,
                    disp: (((x & 0x0f) as usize) << 8 | y as usize) + 1,
                };
                if t.disp > out {
                    return None;
                }
                toks.push((out, t));
                out += t.len;
            } else {
                p += 1;
                toks.push((out, Tok { len: 1, disp: 0 }));
                out += 1;
            }
        }
    }
    // a final match may run past the declared size (the decoder stops there); such streams are not spliced
    (out == size).then_some(Lz77Stream {
        toks,
        groups,
        header,
        end: p,
    })
}

fn lz77_header(n: usize, header: usize) -> Vec<u8> {
    if header == 4 && n > 0 && n < 1 << 24 {
        vec![0x10, n as u8, (n >> 8) as u8, (n >> 16) as u8]
    } else {
        let mut h = vec![0x10, 0, 0, 0];
        h.extend_from_slice(&(n as u32).to_le_bytes());
        h
    }
}

/// Append `toks` (the first one at `data[pos]`) as flag groups of up to 8 tokens.
fn emit_groups(data: &[u8], toks: &[Tok], mut pos: usize, out: &mut Vec<u8>) {
    for group in toks.chunks(8) {
        let flag_at = out.len();
        out.push(0);
        let mut flags = 0u8;
        for (k, t) in group.iter().enumerate() {
            if t.disp == 0 {
                out.push(data[pos]);
            } else {
                flags |= 0x80 >> k;
                let d = t.disp - 1;
                out.push(((t.len - 3) as u8) << 4 | (d >> 8) as u8);
                out.push(d as u8);
            }
            pos += t.len;
        }
        out[flag_at] = flags;
    }
}

/// Does match `t` at position `p` reproduce `data`?
fn valid(data: &[u8], p: usize, t: Tok) -> bool {
    p + t.len <= data.len()
        && t.disp != 0
        && t.disp <= p
        && (0..t.len).all(|i| data[p + i] == data[p + i - t.disp])
}

/// Longest match at `p` (distance 2..=4096, length 3..=`cap`), or a literal.
fn greedy_at(data: &[u8], p: usize, cap: usize) -> Tok {
    let max = 18.min(cap).min(data.len() - p);
    let mut best = Tok { len: 1, disp: 0 };
    if max >= 3 {
        for d in 2..=p.min(0x1000) {
            let mut l = 0;
            while l < max && data[p + l] == data[p + l - d] {
                l += 1;
            }
            if l >= 3 && l > best.len.max(2) {
                best = Tok { len: l, disp: d };
                if l == max {
                    break;
                }
            }
        }
    }
    best
}

/// Above this many changed bytes an edit is a rewrite: [`compress`] is used instead of splicing.
const SPLICE_LIMIT: usize = 256 * 1024;

/// Compress `new` to LZ77 type 0x10 so that it differs from `original` (an LZ77 stream of similar data, e.g. the
/// retail file) as little as possible, so a binary diff against the original stays small.
///
/// The original stream is a sequence of flag groups (a flag byte + 8 tokens). Every group whose tokens all still
/// decode to the same bytes of `new` is copied byte for byte. Each run of groups in between is re-encoded from `new`,
/// reusing the original tokens that are still valid and greedily encoding the rest, then padded to a whole number of
/// groups by splitting matches (into a shorter match plus literals), so the next kept group starts on a group
/// boundary again. A run that cannot be padded absorbs the following kept group and tries again.
///
/// Falls back to [`compress`] when `original` is not a well-formed LZ77 stream or the edit is a rewrite. The
/// result always decompresses to exactly `new` (checked here; on a mismatch the fallback is used).
pub fn recompress(original: &[u8], new: &[u8]) -> Vec<u8> {
    match lz77_splice(original, new) {
        Some(b) if decompress_bounded(&b, new.len()).ok().as_deref() == Some(new) => b,
        _ => compress(new),
    }
}

fn lz77_splice(original: &[u8], new: &[u8]) -> Option<Vec<u8>> {
    let st = lz77_stream(original)?;
    let old = decompress_bounded(original, 64 * 1024 * 1024).ok()?;
    if new.len() as u64 >= 1 << 32 {
        return None;
    }
    // unchanged prefix `a` and suffix `s`; tokens in between are reused only when the size did not change
    let a = old.iter().zip(new).take_while(|(x, y)| x == y).count();
    let s = old[a..]
        .iter()
        .rev()
        .zip(new[a..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let delta = new.len() as isize - old.len() as isize;
    if new.len() - a - s > SPLICE_LIMIT {
        return None;
    }
    // position in `new` of each original token that still decodes to the same bytes there
    let keep: Vec<Option<usize>> = st
        .toks
        .iter()
        .map(|&(q, t)| {
            let p = if q + t.len <= a || delta == 0 {
                q
            } else if q >= old.len() - s {
                (q as isize + delta) as usize
            } else {
                return None;
            };
            let ok = if t.disp == 0 {
                new.get(p) == Some(&old[q])
            } else {
                valid(new, p, t)
            };
            ok.then_some(p)
        })
        .collect();
    let ngroups = st.groups.len();
    let span = |g: usize| 8 * g..(8 * g + 8).min(st.toks.len());
    // start (in `new`) of each group that can be copied verbatim: all its tokens kept, still tiling `new`
    let clean: Vec<Option<usize>> = (0..ngroups)
        .map(|g| {
            let r = span(g);
            let start = keep[r.start]?;
            let mut p = start;
            for k in r {
                if keep[k] != Some(p) {
                    return None;
                }
                p += st.toks[k].1.len;
            }
            // the final, partial group can only stay final: more tokens after it would need its unused flag bits
            (span(g).len() == 8 || p == new.len()).then_some(start)
        })
        .collect();
    let group_bytes = |g: usize| st.groups[g]..st.groups.get(g + 1).copied().unwrap_or(st.end);
    let group_end = |g: usize| clean[g].unwrap() + span(g).map(|k| st.toks[k].1.len).sum::<usize>();
    let next_clean =
        |from: usize, p: usize| (from..ngroups).find(|&h| clean[h].is_some_and(|x| x >= p));

    let mut out = lz77_header(new.len(), st.header);
    let (mut p, mut g) = (0, 0);
    while g < ngroups && p < new.len() {
        if clean[g] == Some(p) {
            out.extend_from_slice(&original[group_bytes(g)]);
            p = group_end(g);
            g += 1;
            continue;
        }
        // re-encode new[p..y) up to the next kept group (or the end), padded to whole groups
        let mut stop = next_clean(g, p);
        loop {
            let y = stop.map_or(new.len(), |h| clean[h].unwrap());
            let reusable = (8 * g..stop.map_or(st.toks.len(), |h| 8 * h))
                .filter_map(|k| Some((keep[k]?, st.toks[k].1)));
            let mut run = encode_run(new, p, y, reusable);
            if stop.is_none() || pad(&mut run) {
                emit_groups(
                    new,
                    &run.iter().map(|r| r.1).collect::<Vec<_>>(),
                    p,
                    &mut out,
                );
                p = y;
                g = stop.unwrap_or(ngroups);
                break;
            }
            stop = next_clean(stop.unwrap() + 1, p);
        }
    }
    if p < new.len() {
        // the original stream ran out before the new data did: encode the rest
        let run = encode_run(new, p, new.len(), std::iter::empty());
        emit_groups(
            new,
            &run.iter().map(|r| r.1).collect::<Vec<_>>(),
            p,
            &mut out,
        );
    }
    out.extend_from_slice(&original[st.end..]); // the original's trailing padding
    Some(out)
}

/// Tokens covering exactly `data[p..y)`: the given still-valid original tokens (sorted by position) where they fit,
/// greedy matches elsewhere.
fn encode_run(
    data: &[u8],
    mut p: usize,
    y: usize,
    kept: impl Iterator<Item = (usize, Tok)>,
) -> Vec<(usize, Tok)> {
    let kept: Vec<(usize, Tok)> = kept.filter(|&(q, t)| q >= p && q + t.len <= y).collect();
    let mut run = Vec::new();
    let mut m = 0;
    while p < y {
        while m < kept.len() && kept[m].0 < p {
            m += 1;
        }
        let t = match kept.get(m) {
            Some(&(q, t)) if q == p => t,
            next => greedy_at(data, p, next.map_or(y, |n| n.0) - p),
        };
        run.push((p, t));
        p += t.len;
    }
    run
}

/// Pad `run` to a whole number of 8-token groups by splitting matches: match(L) -> match(L-k) + k literals, or L
/// literals. False when it has too few matches to split.
fn pad(run: &mut Vec<(usize, Tok)>) -> bool {
    let mut need = (8 - run.len() % 8) % 8;
    let lit = Tok { len: 1, disp: 0 };
    let mut i = 0;
    while need > 0 && i < run.len() {
        let (p, t) = run[i];
        let k = if t.disp == 0 {
            0
        } else if need >= t.len - 1 {
            t.len - 1
        } else {
            need.min(t.len - 3)
        };
        if k > 0 {
            let head = if k == t.len - 1 {
                lit
            } else {
                Tok {
                    len: t.len - k,
                    disp: t.disp,
                }
            };
            let tail = (p + t.len - k..p + t.len).map(|q| (q, lit));
            run.splice(i..=i, std::iter::once((p, head)).chain(tail));
            need -= k;
        }
        i += k + 1;
    }
    need == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lz77_round_trips_and_rejects_bad_data() {
        for data in [
            b"".to_vec(),
            b"abcabcabcabcabcabcabcabc".to_vec(),
            (0..70000u32).map(|i| (i * 7 / 13) as u8).collect(),
            vec![0x42; 9000],
        ] {
            let c = compress(&data);
            assert_eq!(decompress_bounded(&c, 1 << 20).unwrap(), data);
        }
        assert!(decompress_bounded(b"\x11\x04\0\0abcd", 100).is_err());
        assert!(decompress_bounded(b"\x10\x10\0\0\x80\x00\x05", 100).is_err());
        assert!(decompress_bounded(&compress(&[1; 100]), 50).is_err());
    }

    /// Compressible synthetic data (no game content).
    fn sample(entries: u32) -> Vec<u8> {
        let mut data = Vec::new();
        for i in 0..entries {
            data.extend_from_slice(
                format!("entry {} value {}; ", i % 300, i * 7 % 1000).as_bytes(),
            );
        }
        data
    }
    fn noise(n: usize, seed: u32) -> Vec<u8> {
        let mut x = seed.wrapping_mul(2654435761).wrapping_add(12345);
        (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x as u8
            })
            .collect()
    }
    /// An LZ77 stream from a different encoder than ours (token boundaries differ from [`compress`]), standing
    /// in for a retail file.
    fn foreign_lz77(data: &[u8]) -> Vec<u8> {
        let (mut toks, mut p) = (Vec::new(), 0);
        while p < data.len() {
            let t = greedy_at(data, p, 4 + p % 13);
            toks.push(t);
            p += t.len;
        }
        let mut out = lz77_header(data.len(), 4);
        emit_groups(data, &toks, 0, &mut out);
        out.extend_from_slice(&[0, 0]); // trailing padding, kept by recompression
        assert_eq!(decompress_bounded(&out, usize::MAX).unwrap(), data);
        out
    }
    fn patch_len(a: &[u8], b: &[u8]) -> usize {
        let mut patch = Vec::new();
        qbsdiff::Bsdiff::new(a, b).compare(&mut patch).unwrap();
        patch.len()
    }

    #[test]
    fn recompression_keeps_the_original_stream_outside_the_edit() {
        let data = sample(20000);
        for original in [compress(&data), foreign_lz77(&data)] {
            assert_eq!(recompress(&original, &data), original);
            for (at, insert) in [
                (5000usize, 0usize),
                (100000, 0),
                (60000, 5),
                (data.len() - 3, 0),
            ] {
                let mut edited = data.clone();
                edited[at] ^= 0x20;
                for _ in 0..insert {
                    edited.insert(at, b'#');
                }
                let new = recompress(&original, &edited);
                assert_eq!(decompress_bounded(&new, 1 << 24).unwrap(), edited);
                let patch = patch_len(&original, &new);
                assert!(
                    patch < 1000,
                    "edit at {at} (+{insert}): patch {patch} bytes for {}",
                    original.len()
                );
            }
        }
        let original = compress(&data);
        let other: Vec<u8> = (0..5000u32).map(|i| (i * i % 251) as u8).collect();
        assert_eq!(
            decompress_bounded(&recompress(&original, &other), 1 << 20).unwrap(),
            other
        );
        assert_eq!(
            decompress_bounded(&recompress(b"not lz", &other), 1 << 20).unwrap(),
            other
        );
        assert_eq!(
            decompress_bounded(&recompress(&original, b""), 16).unwrap(),
            b""
        );
    }

    #[test]
    fn recompression_splices_incompressible_edits_at_start_middle_and_end() {
        // a 1 KiB block of noise (like a recoloured palette) re-encodes to literals only, which cannot be split to
        // realign the flag groups: the run has to absorb following groups instead of shifting the rest of the stream
        let data = sample(4000);
        let original = foreign_lz77(&data);
        for (at, n) in [
            (0usize, 1024usize),
            (0, 1),
            (data.len() / 2, 1024),
            (data.len() / 2 + 3, 37),
            (data.len() - 1024, 1024),
            (data.len() - 1, 1),
        ] {
            let mut edited = data.clone();
            edited[at..at + n].copy_from_slice(&noise(n, at as u32 + 1));
            let new = recompress(&original, &edited);
            assert_eq!(
                decompress_bounded(&new, 1 << 24).unwrap(),
                edited,
                "edit at {at}+{n}"
            );
            let ours = patch_len(&original, &new);
            let full = patch_len(&original, &compress(&edited));
            // the noise itself is incompressible: about 2 bytes of patch per new byte, plus bsdiff overhead
            assert!(ours < 2 * n + 600, "edit at {at}+{n}: patch {ours} bytes");
            assert!(
                ours * 4 < full,
                "edit at {at}+{n}: patch {ours} bytes vs {full} for full recompression"
            );
        }
    }

    #[test]
    fn recompression_handles_size_changes_and_scattered_edits() {
        let data = sample(4000);
        let original = foreign_lz77(&data);
        let full = |e: &[u8]| patch_len(&original, &compress(e));
        // grow, shrink, and replace with different-length content
        let mid = data.len() / 2;
        let cases: Vec<Vec<u8>> = vec![
            [&data[..mid], &noise(300, 9)[..], &data[mid..]].concat(),
            [&data[..mid], &data[mid + 777..]].concat(),
            [&data[..10], &noise(50, 3)[..], &data[200..]].concat(),
            [&data[..], b"appended tail"].concat(),
            data[..data.len() - 500].to_vec(),
        ];
        for edited in &cases {
            let new = recompress(&original, edited);
            assert_eq!(decompress_bounded(&new, 1 << 24).unwrap(), *edited);
            let ours = patch_len(&original, &new);
            assert!(
                ours < 1500 && ours * 5 < full(edited),
                "size {} -> {}: patch {ours} bytes",
                data.len(),
                edited.len()
            );
        }
        // many small same-size edits scattered through the file (tokens between them are reused)
        let mut edited = data.clone();
        for (i, b) in noise(40, 7).into_iter().enumerate() {
            edited[i * 2311 % data.len()] = b;
        }
        let new = recompress(&original, &edited);
        assert_eq!(decompress_bounded(&new, 1 << 24).unwrap(), edited);
        assert!(patch_len(&original, &new) * 4 < full(&edited));
    }

    #[test]
    fn recompression_round_trips_random_edits() {
        let data = sample(1500);
        for (i, original) in [
            compress(&data),
            foreign_lz77(&data),
            foreign_lz77(&noise(3000, 1)),
        ]
        .into_iter()
        .enumerate()
        {
            let base = decompress_bounded(&original, usize::MAX).unwrap();
            for seed in 0..40u32 {
                let r = noise(4, seed * 31 + i as u32);
                let at = (r[0] as usize * 256 + r[1] as usize) % base.len();
                let cut = (r[2] as usize % 64).min(base.len() - at);
                let ins = noise(r[3] as usize % 64, seed);
                let edited = [&base[..at], &ins[..], &base[at + cut..]].concat();
                let new = recompress(&original, &edited);
                assert_eq!(
                    decompress_bounded(&new, 1 << 24).unwrap(),
                    edited,
                    "stream {i} seed {seed}"
                );
            }
        }
    }
}

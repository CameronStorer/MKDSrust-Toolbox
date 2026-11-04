//! Original Nitro resource dictionary and container encoders.
use crate::{error, put16, put32, Result};
pub(crate) fn align(b: &mut Vec<u8>, n: usize) {
    while !b.len().is_multiple_of(n) {
        b.push(0);
    }
}
pub(crate) fn dictionary(entries: &[(String, Vec<u8>)], stride: usize) -> Result<Vec<u8>> {
    if entries.len() > 255
        || entries
            .iter()
            .any(|(n, d)| n.is_empty() || n.len() > 16 || !n.is_ascii() || d.len() != stride)
    {
        return Err(error(
            0,
            "Nitro dictionary needs unique ASCII names of 1–16 bytes and fixed-size entries",
        ));
    }
    let mut keys = vec![[0u8; 16]];
    for (name, _) in entries {
        let mut key = [0; 16];
        key[..name.len()].copy_from_slice(name.as_bytes());
        if keys.contains(&key) {
            return Err(error(0, "duplicate Nitro resource name"));
        }
        keys.push(key);
    }
    // Patricia nodes: bit index, left, right, entry index. Links back to
    // equal/higher bit indices terminate the search at that node's key.
    let mut nodes = vec![[127u8, 0, 0, 0]];
    let bit = |k: &[u8; 16], at: u8| -> usize { ((k[at as usize / 8] >> (at % 8)) & 1) as usize };
    for i in 1..keys.len() {
        let mut parent = 0usize;
        let mut leaf = nodes[0][1] as usize;
        while nodes[parent][0] > nodes[leaf][0] {
            parent = leaf;
            leaf = nodes[leaf][1 + bit(&keys[i], nodes[leaf][0])] as usize;
        }
        let diff = (0..127u8)
            .rev()
            .find(|&b| bit(&keys[i], b) != bit(&keys[leaf], b))
            .ok_or_else(|| error(0, "duplicate dictionary key"))?;
        let mut parent = 0usize;
        let mut child = nodes[0][1] as usize;
        while nodes[parent][0] > nodes[child][0] && nodes[child][0] > diff {
            parent = child;
            child = nodes[child][1 + bit(&keys[i], nodes[child][0])] as usize;
        }
        let side = bit(&keys[i], diff);
        let mut node = [diff, child as u8, child as u8, (i - 1) as u8];
        node[1 + side] = i as u8;
        nodes.push(node);
        let link = if parent == 0 {
            1
        } else {
            1 + bit(&keys[i], nodes[parent][0])
        };
        nodes[parent][link] = i as u8;
    }
    let table = 8 + nodes.len() * 4;
    let names = table + 4 + entries.len() * stride;
    let len = names + entries.len() * 16;
    if len > 65535 {
        return Err(error(0, "dictionary exceeds 64 KiB"));
    }
    let mut b = vec![0; len];
    b[1] = entries.len() as u8;
    put16(&mut b, 2, len as u16);
    put16(&mut b, 4, 8);
    put16(&mut b, 6, table as u16);
    for (i, n) in nodes.iter().enumerate() {
        b[8 + i * 4..12 + i * 4].copy_from_slice(n);
    }
    put16(&mut b, table, stride as u16);
    put16(&mut b, table + 2, (names - table) as u16);
    for (i, (_, data)) in entries.iter().enumerate() {
        b[table + 4 + i * stride..table + 4 + (i + 1) * stride].copy_from_slice(data);
        b[names + i * 16..names + (i + 1) * 16].copy_from_slice(&keys[i + 1]);
    }
    Ok(b)
}
pub(crate) fn container(magic: &[u8; 4], blocks: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![0; 16 + blocks.len() * 4];
    b[..4].copy_from_slice(magic);
    put16(&mut b, 4, 0xfeff);
    put16(&mut b, 6, 2);
    put16(&mut b, 12, 16);
    put16(&mut b, 14, blocks.len() as u16);
    for (i, block) in blocks.iter().enumerate() {
        align(&mut b, 4);
        let at = b.len();
        put32(&mut b, 16 + i * 4, at as u32);
        b.extend(block);
    }
    let len = b.len();
    put32(&mut b, 8, len as u32);
    b
}
#[cfg(test)]
mod tests {
    #[test]
    fn resource_tree_lookup_finds_every_key() {
        let entries = (0u32..100)
            .map(|i| (format!("resource{i}"), i.to_le_bytes().to_vec()))
            .collect::<Vec<_>>();
        let b = super::dictionary(&entries, 4).unwrap();
        for (index, (name, _)) in entries.iter().enumerate() {
            let mut key = [0; 16];
            key[..name.len()].copy_from_slice(name.as_bytes());
            let mut parent = 0;
            let mut node = b[9] as usize;
            for _ in 0..128 {
                let bit = b[8 + node * 4];
                if b[8 + parent * 4] <= bit {
                    break;
                }
                parent = node;
                let side = ((key[bit as usize / 8] >> (bit % 8)) & 1) as usize;
                node = b[8 + node * 4 + 1 + side] as usize;
            }
            assert_eq!(b[8 + node * 4 + 3] as usize, index);
        }
        assert_eq!(crate::texture::dictionary(&b, 0, 4).unwrap().len(), 100);
    }
}

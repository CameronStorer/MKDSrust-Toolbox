use nitro_formats::{
    audio::*,
    graphics::{Ncgr, Nclr},
};
#[test]
fn compact_palette_and_legacy_character_headers_roundtrip() {
    // Compact palette: an expanded 15-bank address space, one stored bank,
    // and a PCMP slot map. Synthetic colours only.
    let mut b = Nclr::new(&[0x1234; 256], true, true).write();
    b[32..36].copy_from_slice(&7680u32.to_le_bytes());
    let end = b.len();
    b.extend(b"PMCP");
    b.extend(18u32.to_le_bytes());
    b.extend([1, 0, 0, 0, 8, 0, 0, 0, 14, 0]);
    let len = b.len();
    b[8..12].copy_from_slice(&(len as u32).to_le_bytes());
    b[14..16].copy_from_slice(&2u16.to_le_bytes());
    let mut palette = Nclr::parse(&b).unwrap();
    assert_eq!(palette.colors.len(), 256);
    assert_eq!(palette.write(), b);
    palette.colors[0] ^= 1;
    let out = palette.write();
    assert_eq!(&out[end..], &b[end..]);
    let mut b = Ncgr::new(1, 1, 4, false, vec![1; 64]).unwrap().write();
    b[4..6].fill(0);
    let len = b.len();
    b[8..12].copy_from_slice(&((len - 8) as u32).to_le_bytes());
    let block = u32::from_le_bytes(b[20..24].try_into().unwrap());
    b[20..24].copy_from_slice(&(block - 8).to_le_bytes());
    let parsed = Ncgr::parse(&b).unwrap();
    assert_eq!(parsed.pixels.len(), 64);
    assert_eq!(parsed.write(), b);
}
#[test]
fn mutable_archive_and_stream_writers_and_truncation_errors() {
    let mut ssar = Ssar::new(
        vec![SequenceArchiveEntry {
            offset: Some(0),
            bank: 0,
            volume: 100,
            priorities: [64, 64, 0],
        }],
        vec![0xff],
    )
    .unwrap();
    ssar.entries[0].volume = 80;
    ssar.events.push(0xff);
    assert_eq!(
        Ssar::parse(&ssar.write().unwrap()).unwrap().entries[0].volume,
        80
    );
    let mut stream = Strm::from_pcm(&[vec![100; 128]], 16000, None).unwrap();
    stream.sample_rate = 8000;
    stream.loop_sample = Some(4);
    let bytes = stream.write().unwrap();
    let stream = Strm::parse(&bytes).unwrap();
    assert_eq!(stream.sample_rate, 8000);
    assert_eq!(stream.loop_sample, Some(4));
    let wave = Swav::from_pcm(&[123; 128], 16000, 2, Some(0))
        .unwrap()
        .write()
        .unwrap();
    for end in 0..wave.len() {
        assert!(Swav::parse(&wave[..end]).is_err());
    }
    let mut bad = ssar.write().unwrap();
    bad[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Ssar::parse(&bad).is_err());
}

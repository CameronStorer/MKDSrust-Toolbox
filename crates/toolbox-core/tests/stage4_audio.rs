use nitro_formats::audio::*;
use toolbox_core::audio::{self, Pcm};
fn pcm() -> Pcm {
    Pcm {
        rate: 22050,
        channels: vec![(0..22050)
            .map(|i| ((i as f32 / 30.).sin() * 16000.) as i16)
            .collect()],
    }
}
fn archive() -> Sdat {
    let song = audio::sampled(&pcm(), 16000, false, 2, Some(2000), Some(21000)).unwrap();
    let mut s = Sdat::default();
    s.files = vec![
        song.sequence.write(),
        song.bank.write().unwrap(),
        song.waves.write().unwrap(),
    ];
    s.info[0].push(Some(vec![0, 0, 0, 0, 0, 0, 127, 64, 64, 0]));
    s.info[2].push(Some(vec![1, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255]));
    s.info[3].push(Some(vec![2, 0, 0, 0]));
    s
}
#[test]
fn wav_codecs_sampled_route_sdat_and_mixer() {
    let source = pcm();
    let wav = audio::wav(&source).unwrap();
    let decoded = audio::read_wav(&wav).unwrap();
    assert_eq!(decoded.channels, source.channels);
    for encoding in 0..=2 {
        let s = Swav::from_pcm(&source.channels[0], 22050, encoding, Some(2000)).unwrap();
        let b = s.write().unwrap();
        let s = Swav::parse(&b).unwrap();
        assert_eq!(s.write().unwrap(), b);
        let samples = s.samples().unwrap();
        assert!(samples.len() >= source.channels[0].len());
        let error = samples
            .iter()
            .zip(&source.channels[0])
            .map(|(a, b)| (*a as f64 - *b as f64).abs())
            .sum::<f64>()
            / source.channels[0].len() as f64;
        assert!(error < 1000., "encoding {encoding}, error {error}");
    }
    let mut s = archive();
    let b = s.write().unwrap();
    s = Sdat::parse(&b).unwrap();
    s.validate_files().unwrap();
    assert_eq!(s.write().unwrap(), b);
    let mix = audio::preview_sequence(&s, 0, 0.5).unwrap();
    assert!(mix.channels[0].iter().any(|v| v.abs() > 100));
    let song = audio::sampled(&source, 8000, false, 1, None, None).unwrap();
    assert!(song.ram_bytes > 16000);
    s.replace_sampled(0, song.sequence, song.bank, song.waves)
        .unwrap();
    let s = Sdat::parse(&s.write().unwrap()).unwrap();
    assert_eq!(s.info[2].len(), 2);
    assert_eq!(s.bank_for_sequence(0).unwrap(), 1);
    s.validate_files().unwrap();
}
#[test]
fn midi_timing_program_controllers_bend_and_loop_markers() {
    let mut track = vec![
        0, 0xc0, 10, 0, 0xb0, 7, 100, 0, 0xb0, 10, 32, 0, 0xe0, 0, 80, 0, 0xff, 0x51, 3, 7, 0xa1,
        0x20, 0, 0xff, 6, 9,
    ];
    track.extend(b"loopStart");
    track.extend([0, 0x90, 60, 100, 96, 0x80, 60, 0, 0, 0xff, 6, 7]);
    track.extend(b"loopEnd");
    track.extend([0, 0xff, 0x2f, 0]);
    let mut midi = b"MThd\0\0\0\x06\0\0\0\x01\0\x60MTrk".to_vec();
    midi.extend((track.len() as u32).to_be_bytes());
    midi.extend(track);
    let mut map = [0; 128];
    map[10] = 3;
    let seq = audio::midi(&midi, &map).unwrap();
    let t = timeline(&seq.events, 200).unwrap();
    assert!(t.notes.len() > 1);
    let n = &t.notes[0];
    assert_eq!(n.program, 3);
    assert_eq!(n.duration, 48);
    assert_eq!(n.volume, 100);
    assert_eq!(n.pan, 32);
    assert!(n.bend > 0.);
    assert_eq!(Sseq::parse(&seq.write()).unwrap().events, seq.events);
    assert!(audio::midi(b"bad", &map).is_err());
}
#[test]
fn stereo_stream_and_sequence_archive() {
    let source = pcm();
    let channels = vec![
        source.channels[0].clone(),
        source.channels[0].iter().map(|s| -s).collect(),
    ];
    let stream = Strm::from_pcm(&channels, 22050, Some(100)).unwrap();
    let stream = Strm::parse(&stream.write().unwrap()).unwrap();
    assert_eq!(stream.pcm().unwrap(), channels);
    let ssar = Ssar::new(
        vec![SequenceArchiveEntry {
            offset: Some(0),
            bank: 0,
            volume: 127,
            priorities: [64, 64, 0],
        }],
        vec![0xff],
    )
    .unwrap();
    assert_eq!(
        Ssar::parse(&ssar.write().unwrap()).unwrap().entries.len(),
        1
    );
    assert!(Swav::parse(b"bad").is_err());
    assert!(Sdat::parse(b"bad").is_err());
}

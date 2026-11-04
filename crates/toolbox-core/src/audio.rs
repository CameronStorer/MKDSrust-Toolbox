//! Authored WAV/MIDI conversion and a bounded, deliberately basic software mixer.
use crate::{Error, Result};
use nitro_formats::audio::{self, Instrument, Sbnk, Sdat, Sseq, Swar, Swav};
use serde::Serialize;
use specta::Type;
fn invalid(s: &str) -> Error {
    Error::Invalid(s.into())
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct AudioResource {
    pub kind: u8,
    pub index: u32,
    pub name: String,
    pub file: u32,
    pub bytes: u32,
    pub samples: u32,
}
pub fn resources(sdat: &Sdat) -> Result<Vec<AudioResource>> {
    let mut out = vec![];
    for kind in [0, 1, 2, 3, 7] {
        for (i, r) in sdat.info[kind].iter().enumerate() {
            if r.is_none() {
                continue;
            }
            let file = sdat.resource_file(kind, i)?;
            let b = &sdat.files[file];
            let samples = if kind == 3 {
                Swar::parse(b)?.waves.len() as u32
            } else {
                0
            };
            out.push(AudioResource {
                kind: kind as u8,
                index: i as u32,
                name: sdat.symbols[kind]
                    .get(i)
                    .and_then(Clone::clone)
                    .unwrap_or_else(|| {
                        format!(
                            "{} {i}",
                            [
                                "Sequence",
                                "Sequence archive",
                                "Bank",
                                "Wave archive",
                                "Player",
                                "Group",
                                "Stream player",
                                "Stream"
                            ][kind]
                        )
                    }),
                file: file as u32,
                bytes: b.len() as u32,
                samples,
            });
        }
    }
    Ok(out)
}
#[derive(Clone, Debug)]
pub struct Pcm {
    pub rate: u32,
    pub channels: Vec<Vec<i16>>,
}
pub fn read_wav(b: &[u8]) -> Result<Pcm> {
    if b.get(..4) != Some(b"RIFF") || b.get(8..12) != Some(b"WAVE") {
        return Err(invalid("import a RIFF WAV file"));
    }
    let mut at = 12;
    let mut format = None;
    let mut pcm = None;
    while at + 8 <= b.len() {
        let n = u32::from_le_bytes(b[at + 4..at + 8].try_into().unwrap()) as usize;
        let data = b
            .get(at + 8..at + 8 + n)
            .ok_or_else(|| invalid("truncated WAV chunk"))?;
        match &b[at..at + 4] {
            b"fmt " => {
                if n < 16 {
                    return Err(invalid("truncated WAV format"));
                }
                format = Some((
                    u16::from_le_bytes(data[..2].try_into().unwrap()),
                    u16::from_le_bytes(data[2..4].try_into().unwrap()) as usize,
                    u32::from_le_bytes(data[4..8].try_into().unwrap()),
                    u16::from_le_bytes(data[14..16].try_into().unwrap()) as usize,
                ));
            }
            b"data" => pcm = Some(data),
            _ => {}
        }
        at += 8 + n + (n & 1);
    }
    let (kind, channels, rate, bits) = format.ok_or_else(|| invalid("WAV has no format chunk"))?;
    let data = pcm.ok_or_else(|| invalid("WAV has no audio data"))?;
    if !(1..=2).contains(&channels)
        || rate == 0
        || rate > 192000
        || ![(1, 8), (1, 16), (3, 32)].contains(&(kind, bits))
    {
        return Err(invalid("WAV must be mono/stereo PCM8, PCM16 or float32"));
    }
    let bytes = bits / 8;
    if !data.len().is_multiple_of(bytes * channels) {
        return Err(invalid("WAV has a partial frame"));
    }
    let mut out = vec![vec![]; channels];
    for frame in data.chunks_exact(bytes * channels) {
        for c in 0..channels {
            let d = &frame[c * bytes..(c + 1) * bytes];
            let value = match (kind, bits) {
                (1, 8) => (d[0] as i16 - 128) << 8,
                (1, 16) => i16::from_le_bytes(d.try_into().unwrap()),
                _ => {
                    let n = f32::from_le_bytes(d.try_into().unwrap());
                    if !n.is_finite() {
                        return Err(invalid("non-finite WAV sample"));
                    }
                    (n.clamp(-1., 1.) * 32767.) as i16
                }
            };
            out[c].push(value);
        }
    }
    if out[0].is_empty() || out[0].len() > rate as usize * 600 {
        return Err(invalid("WAV must contain up to ten minutes of audio"));
    }
    Ok(Pcm {
        rate,
        channels: out,
    })
}
pub fn wav(pcm: &Pcm) -> Result<Vec<u8>> {
    if pcm.channels.is_empty()
        || pcm.channels.len() > 2
        || pcm
            .channels
            .iter()
            .any(|c| c.len() != pcm.channels[0].len())
    {
        return Err(invalid("invalid PCM channels"));
    }
    let channels = pcm.channels.len() as u16;
    let len = pcm.channels[0].len() * channels as usize * 2;
    let mut b = b"RIFF".to_vec();
    b.extend((36 + len as u32).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(channels.to_le_bytes());
    b.extend(pcm.rate.to_le_bytes());
    b.extend((pcm.rate * channels as u32 * 2).to_le_bytes());
    b.extend((channels * 2).to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend((len as u32).to_le_bytes());
    for i in 0..pcm.channels[0].len() {
        for c in &pcm.channels {
            b.extend(c[i].to_le_bytes());
        }
    }
    Ok(b)
}
pub struct SampledSong {
    pub sequence: Sseq,
    pub bank: Sbnk,
    pub waves: Swar,
    pub ram_bytes: usize,
    pub loop_sample: Option<usize>,
}
pub fn sampled(
    pcm: &Pcm,
    rate: u16,
    stereo: bool,
    encoding: u8,
    loop_start: Option<u32>,
    loop_end: Option<u32>,
) -> Result<SampledSong> {
    if !(8000..=48000).contains(&rate)
        || pcm.channels.is_empty()
        || pcm.channels.len() > 2
        || pcm.rate == 0
        || pcm
            .channels
            .iter()
            .any(|c| c.len() != pcm.channels[0].len())
    {
        return Err(invalid("invalid sample conversion settings"));
    }
    let end = loop_end.map_or(pcm.channels[0].len(), |n| n as usize);
    if end == 0 || end > pcm.channels[0].len() || loop_start.is_some_and(|n| n as usize >= end) {
        return Err(invalid("loop points must lie within the source WAV"));
    }
    let count = end * rate as usize / pcm.rate as usize;
    if count == 0 || count > 8 * 1024 * 1024 {
        return Err(invalid(
            "sampled song exceeds the conversion RAM limit; use MIDI or lower quality",
        ));
    }
    let channels = if stereo && pcm.channels.len() == 2 {
        2
    } else {
        1
    };
    let mut waves = vec![];
    for c in 0..channels {
        let mut samples = Vec::with_capacity(count);
        for i in 0..count {
            let pos = i as f64 * pcm.rate as f64 / rate as f64;
            let a = (pos as usize).min(end - 1);
            let b = (a + 1).min(end - 1);
            let read = |i| {
                if channels == 1 && pcm.channels.len() == 2 {
                    (pcm.channels[0][i] as i32 + pcm.channels[1][i] as i32) as f32 / 2.
                } else {
                    pcm.channels[c][i] as f32
                }
            };
            samples.push((read(a) + (read(b) - read(a)) * (pos - a as f64) as f32).round() as i16);
        }
        waves.push(Swav::from_pcm(
            &samples,
            rate,
            encoding,
            loop_start.map(|n| n as usize * rate as usize / pcm.rate as usize),
        )?);
    }
    let actual_loop = waves[0].looped.then(|| waves[0].loop_sample());
    let mut events = vec![0xfe, if channels == 2 { 3 } else { 1 }, 0];
    if channels == 2 {
        events.extend([0x93, 1, 0, 0, 0]);
    }
    let mut tracks = Vec::<Vec<u8>>::new();
    for c in 0..channels {
        let mut track = vec![
            0x81,
            c as u8,
            0xc7,
            1,
            0xc0,
            if channels == 1 {
                64
            } else if c == 0 {
                0
            } else {
                127
            },
            60,
            127,
        ];
        track.extend(audio::variable(if actual_loop.is_some() {
            0
        } else {
            ((count as u64 * 96).div_ceil(rate as u64)) as u32
        }));
        if actual_loop.is_some() {
            let wait_offset =
                events.len() + tracks.iter().map(Vec::len).sum::<usize>() + track.len();
            track.extend([
                0x80,
                0x83,
                0x60,
                0x94,
                (wait_offset & 255) as u8,
                ((wait_offset >> 8) & 255) as u8,
                ((wait_offset >> 16) & 255) as u8,
            ]);
        }
        track.push(0xff);
        if c == 1 {
            let at = events.len() + tracks[0].len();
            events[5] = at as u8;
            events[6] = (at >> 8) as u8;
            events[7] = (at >> 16) as u8;
        }
        tracks.push(track);
    }
    for t in tracks {
        events.extend(t);
    }
    let sequence = Sseq::new(events);
    let bank = Sbnk::new(
        (0..channels)
            .map(|c| Instrument::sample(c as u16, 64))
            .collect(),
    );
    let waves = Swar::new(waves);
    let ram_bytes = sequence.write().len() + bank.write()?.len() + waves.write()?.len();
    Ok(SampledSong {
        sequence,
        bank,
        waves,
        ram_bytes,
        loop_sample: actual_loop,
    })
}
#[derive(Clone, Debug)]
struct MidiEvent {
    tick: u32,
    status: u8,
    a: u8,
    b: u8,
    tempo: Option<u32>,
    marker: Option<String>,
}
fn midi_var(b: &[u8], at: &mut usize) -> Result<u32> {
    let mut n = 0;
    for _ in 0..4 {
        let v = *b
            .get(*at)
            .ok_or_else(|| invalid("truncated MIDI variable"))?;
        *at += 1;
        n = (n << 7) | (v & 127) as u32;
        if v & 128 == 0 {
            return Ok(n);
        }
    }
    Err(invalid("MIDI variable too long"))
}
pub fn midi(b: &[u8], program_map: &[u16; 128]) -> Result<Sseq> {
    if b.get(..4) != Some(b"MThd") || b.len() < 14 {
        return Err(invalid("invalid MIDI header"));
    }
    let header = u32::from_be_bytes(b[4..8].try_into().unwrap()) as usize;
    if header < 6 || header + 8 > b.len() {
        return Err(invalid("invalid MIDI header size"));
    }
    let format = u16::from_be_bytes(b[8..10].try_into().unwrap());
    let n = u16::from_be_bytes(b[10..12].try_into().unwrap()) as usize;
    let division = u16::from_be_bytes(b[12..14].try_into().unwrap()) as u32;
    if format > 1 || n > 128 || division == 0 || division & 0x8000 != 0 {
        return Err(invalid("MIDI supports format 0/1 with PPQN timing"));
    }
    let mut at = 8 + header;
    let mut events = vec![];
    for _ in 0..n {
        if b.get(at..at + 4) != Some(b"MTrk") {
            return Err(invalid("missing MIDI track"));
        }
        let size = b
            .get(at + 4..at + 8)
            .ok_or_else(|| invalid("truncated MIDI track"))?;
        let len = u32::from_be_bytes(size.try_into().unwrap()) as usize;
        let track = b
            .get(at + 8..at + 8 + len)
            .ok_or_else(|| invalid("truncated MIDI track data"))?;
        at += 8 + len;
        let mut p = 0;
        let mut tick = 0u32;
        let mut running = 0;
        while p < track.len() {
            tick = tick
                .checked_add(midi_var(track, &mut p)?)
                .ok_or_else(|| invalid("MIDI duration overflow"))?;
            let first = *track
                .get(p)
                .ok_or_else(|| invalid("truncated MIDI event"))?;
            let status = if first & 128 != 0 {
                p += 1;
                first
            } else {
                running
            };
            if status < 128 {
                return Err(invalid("invalid MIDI running status"));
            }
            if status == 0xff {
                running = 0;
                let kind = *track
                    .get(p)
                    .ok_or_else(|| invalid("truncated MIDI meta event"))?;
                p += 1;
                let len = midi_var(track, &mut p)? as usize;
                let data = track
                    .get(p..p + len)
                    .ok_or_else(|| invalid("truncated MIDI metadata"))?;
                p += len;
                if kind == 0x51 && len == 3 {
                    events.push(MidiEvent {
                        tick,
                        status: 0,
                        a: 0,
                        b: 0,
                        tempo: Some(
                            (data[0] as u32) << 16 | (data[1] as u32) << 8 | data[2] as u32,
                        ),
                        marker: None,
                    });
                }
                if kind == 6 {
                    events.push(MidiEvent {
                        tick,
                        status: 0,
                        a: 0,
                        b: 0,
                        tempo: None,
                        marker: Some(String::from_utf8_lossy(data).to_ascii_lowercase()),
                    });
                }
                if kind == 0x2f {
                    break;
                }
            } else if status == 0xf0 || status == 0xf7 {
                running = 0;
                let len = midi_var(track, &mut p)? as usize;
                track
                    .get(p..p + len)
                    .ok_or_else(|| invalid("truncated MIDI SysEx"))?;
                p += len;
            } else {
                if status >= 0xf0 {
                    return Err(invalid("unsupported MIDI system event"));
                }
                running = status;
                let count = if status & 0xe0 == 0xc0 { 1 } else { 2 };
                let data = track
                    .get(p..p + count)
                    .ok_or_else(|| invalid("truncated MIDI channel event"))?;
                p += count;
                if data.iter().any(|v| *v >= 128) {
                    return Err(invalid("invalid MIDI data byte"));
                }
                events.push(MidiEvent {
                    tick,
                    status,
                    a: data[0],
                    b: data.get(1).copied().unwrap_or(0),
                    tempo: None,
                    marker: None,
                });
            }
            if events.len() > 250_000 {
                return Err(invalid("MIDI event count exceeds limit"));
            }
        }
    }
    events.sort_by_key(|e| e.tick);
    let scale =
        |tick: u32| -> u32 { ((tick as u64 * 48 + division as u64 / 2) / division as u64) as u32 };
    let mut channels = std::collections::BTreeSet::new();
    for e in &events {
        if e.status >= 128 {
            channels.insert(e.status & 15);
        }
    }
    if channels.is_empty() {
        return Err(invalid("MIDI has no channel events"));
    }
    let mut tracks = vec![];
    for (track_id, &channel) in channels.iter().enumerate() {
        let mut commands = vec![(0u32, vec![0xc7, 0])];
        let mut notes = std::collections::BTreeMap::<u8, Vec<(u32, u8)>>::new();
        for e in &events {
            let tick = scale(e.tick);
            if track_id == 0 {
                if let Some(us) = e.tempo {
                    if us == 0 {
                        return Err(invalid("MIDI tempo must be positive"));
                    }
                    let bpm = (60_000_000 / us).clamp(1, 65535) as u16;
                    let mut c = vec![0xe1];
                    c.extend(bpm.to_le_bytes());
                    commands.push((tick, c));
                }
            }
            if let Some(marker) = &e.marker {
                let m = marker.replace([' ', '_', '-'], "");
                if m == "loopstart" {
                    commands.push((tick, vec![0xd4, 0]));
                } else if m == "loopend" {
                    commands.push((tick, vec![0xfc]));
                }
                continue;
            }
            if e.status & 15 != channel || e.status < 128 {
                continue;
            }
            match e.status & 0xf0 {
                0x90 if e.b > 0 => notes.entry(e.a).or_default().push((tick, e.b)),
                0x80 | 0x90 => {
                    if let Some((start, velocity)) = notes.entry(e.a).or_default().pop() {
                        let mut c = vec![e.a, velocity];
                        c.extend(audio::variable((tick - start).max(1)));
                        commands.push((start, c));
                    }
                }
                0xc0 => {
                    let mut c = vec![0x81];
                    c.extend(audio::variable(program_map[e.a as usize] as u32));
                    commands.push((tick, c));
                }
                0xb0 => {
                    if let Some(op) = match e.a {
                        7 => Some(0xc1),
                        10 => Some(0xc0),
                        11 => Some(0xd5),
                        _ => None,
                    } {
                        commands.push((tick, vec![op, e.b]));
                    }
                }
                0xe0 => {
                    let bend =
                        (((e.a as i32 | ((e.b as i32) << 7)) - 8192) / 64).clamp(-128, 127) as i8;
                    commands.push((tick, vec![0xc4, bend as u8]));
                }
                _ => {}
            }
        }
        let end = events.last().map_or(1, |e| scale(e.tick).max(1));
        for (key, pending) in notes {
            for (start, velocity) in pending {
                let mut c = vec![key, velocity];
                c.extend(audio::variable((end - start).max(1)));
                commands.push((start, c));
            }
        }
        commands.sort_by_key(|(t, _)| *t);
        let mut bytes = vec![];
        let mut last = 0;
        for (tick, c) in commands {
            if tick > last {
                bytes.push(0x80);
                bytes.extend(audio::variable(tick - last));
                last = tick;
            }
            bytes.extend(c);
        }
        bytes.extend([0x80]);
        bytes.extend(audio::variable(end.saturating_sub(last).max(1)));
        bytes.push(0xff);
        tracks.push(bytes);
    }
    let mask = (1u32 << tracks.len()) - 1;
    let mut bytes = vec![0xfe, mask as u8, (mask >> 8) as u8];
    let header = 3 + (tracks.len() - 1) * 5;
    let mut offset = header + tracks[0].len();
    for (i, t) in tracks.iter().enumerate().skip(1) {
        bytes.extend([
            0x93,
            i as u8,
            offset as u8,
            (offset >> 8) as u8,
            (offset >> 16) as u8,
        ]);
        offset += t.len();
    }
    if offset > 0xffffff {
        return Err(invalid("SSEQ exceeds branch range"));
    }
    for t in tracks {
        bytes.extend(t);
    }
    Ok(Sseq::new(bytes))
}
pub fn program_map(bank: &Sbnk) -> Result<[u16; 128]> {
    let valid = bank
        .instruments
        .iter()
        .enumerate()
        .filter(|(_, i)| !i.regions.is_empty())
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if valid.is_empty() {
        return Err(invalid("chosen bank contains no instruments"));
    }
    Ok(std::array::from_fn(|program| {
        *valid.iter().min_by_key(|&&i| i.abs_diff(program)).unwrap() as u16
    }))
}
fn seconds(tick: u32, tempos: &[(u32, u16)]) -> f64 {
    let mut time = 0.;
    let mut last = 0;
    let mut bpm = 120;
    for &(at, next) in tempos.iter().take_while(|(at, _)| *at <= tick) {
        time += (at - last) as f64 * 60. / (bpm as f64 * 48.);
        last = at;
        bpm = next.max(1);
    }
    time + (tick - last) as f64 * 60. / (bpm as f64 * 48.)
}
pub fn preview_sequence(sdat: &Sdat, index: usize, duration: f32) -> Result<Pcm> {
    let bank_id = sdat.bank_for_sequence(index)?;
    let bank = Sbnk::parse(&sdat.files[sdat.resource_file(2, bank_id)?])?;
    let ids = sdat.waves_for_bank(bank_id)?;
    let waves = ids
        .iter()
        .map(|&id| {
            if id == 65535 {
                Ok(None)
            } else {
                Ok(Some(Swar::parse(
                    &sdat.files[sdat.resource_file(3, id as usize)?],
                )?))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let sequence = Sseq::parse(&sdat.files[sdat.resource_file(0, index)?])?;
    mix(&sequence, &bank, &waves, duration)
}
pub fn mix(seq: &Sseq, bank: &Sbnk, waves: &[Option<Swar>], duration: f32) -> Result<Pcm> {
    let duration = duration.clamp(0.1, 30.);
    let rate = 22050u32;
    let count = (duration * rate as f32) as usize;
    let mut pcm = vec![[0f32; 2]; count];
    let timeline = audio::timeline(&seq.events, 48000)?;
    let mut active = Vec::<(usize, usize)>::new();
    let mut work = 0usize;
    for event in &timeline.notes {
        let start = (seconds(event.tick, &timeline.tempos) * rate as f64) as usize;
        if start >= count {
            continue;
        }
        let end = ((seconds(event.tick.saturating_add(event.duration), &timeline.tempos)
            * rate as f64) as usize)
            .min(count);
        active.retain(|(_, end)| *end > start);
        if active.len() >= 16 {
            continue;
        }
        active.push((start, end));
        let Some(note) = bank
            .instruments
            .get(event.program as usize)
            .and_then(|i| i.note(event.key))
        else {
            continue;
        };
        let wave = waves
            .get(note.archive as usize)
            .and_then(Option::as_ref)
            .and_then(|s| s.waves.get(note.wave as usize));
        let decoded = if note.kind == 1 {
            Some(
                wave.ok_or_else(|| invalid("sequence sample is missing"))?
                    .samples()?,
            )
        } else {
            None
        };
        let ratio = 2f64.powf((event.key as f64 + event.bend as f64 - note.key as f64) / 12.);
        let pan = ((event.pan as f32 + note.pan as f32 - 64.) / 127.).clamp(0., 1.);
        let gain = event.velocity as f32 / 127. * event.volume as f32 / 127. * 0.25;
        for (i, frame) in pcm.iter_mut().enumerate().take(end).skip(start) {
            work += 1;
            if work > 30_000_000 {
                return Err(invalid("audio preview work limit exceeded"));
            }
            let t = (i - start) as f64 / rate as f64;
            let value = if let (Some(wave), Some(samples)) = (wave, decoded.as_ref()) {
                let mut p = (t * wave.sample_rate as f64 * ratio) as usize;
                if p >= samples.len() {
                    if !wave.looped {
                        break;
                    }
                    let loop_at = wave.loop_sample().min(samples.len().saturating_sub(1));
                    p = loop_at + (p - loop_at) % (samples.len() - loop_at);
                }
                samples[p] as f32 / 32768.
            } else if note.kind == 2 {
                if (t * 440. * 2f64.powf((event.key as f64 - 69.) / 12.)).fract()
                    < (note.wave.min(6) + 1) as f64 / 8.
                {
                    0.5
                } else {
                    -0.5
                }
            } else {
                0.
            };
            frame[0] += value * gain * (1. - pan);
            frame[1] += value * gain * pan;
        }
    }
    Ok(Pcm {
        rate,
        channels: (0..2)
            .map(|channel| {
                pcm.iter()
                    .map(|frame| (frame[channel].clamp(-1., 1.) * 32767.) as i16)
                    .collect()
            })
            .collect(),
    })
}

//! Dev helper: print the voice sequence archive's banks and wave archives per 14-entry block.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let s = toolbox_core::session::RomSession::open(std::path::Path::new(&a[1])).unwrap();
    let d =
        nitro_formats::audio::Sdat::parse(s.file("data/Sound/sound_data.sdat").unwrap()).unwrap();
    for arc in 0..d.info[1].len() {
        let Ok(f) = d.resource_file(1, arc) else {
            continue;
        };
        let Ok(v) = nitro_formats::audio::Ssar::parse(&d.files[f]) else {
            continue;
        };
        println!(
            "seqarc {arc}: {} entries {:?}",
            v.entries.len(),
            d.symbols[1].get(arc)
        );
    }
    let v = nitro_formats::audio::Ssar::parse(&d.files[d.resource_file(1, 2).unwrap()]).unwrap();
    for (k, chunk) in v.entries.chunks(14).enumerate() {
        let banks: Vec<usize> = chunk.iter().map(|e| e.bank as usize).collect();
        let wars: Vec<_> = banks.iter().map(|b| d.waves_for_bank(*b).ok()).collect();
        let war = wars[0].unwrap()[0] as usize;
        let w =
            nitro_formats::audio::Swar::parse(&d.files[d.resource_file(3, war).unwrap()]).unwrap();
        let mut pitches = vec![];
        let mut secs = 0.;
        for wave in &w.waves {
            let x: Vec<f32> = wave.samples().unwrap().iter().map(|v| *v as f32).collect();
            let rate = wave.sample_rate as f32;
            secs += x.len() as f32 / rate;
            let n = (rate * 0.04) as usize;
            for f in x.chunks_exact(n) {
                let energy: f32 = f.iter().map(|v| v * v).sum::<f32>() / n as f32;
                if energy.sqrt() < 1500. {
                    continue;
                }
                let (lo, hi) = ((rate / 600.) as usize, (rate / 70.) as usize);
                let r0: f32 = f.iter().map(|v| v * v).sum();
                let best = (lo..hi.min(n / 2))
                    .map(|l| (l, (0..n - l).map(|i| f[i] * f[i + l]).sum::<f32>() / r0))
                    .max_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((l, c)) = best {
                    if c > 0.4 {
                        pitches.push(rate / l as f32);
                    }
                }
            }
        }
        pitches.sort_by(|a, b| a.total_cmp(b));
        let med = pitches.get(pitches.len() / 2).copied().unwrap_or(0.);
        println!("block {k:2}: bank {} war {war} waves {:3} {secs:5.1}s median pitch {med:5.0} Hz ({} frames)", banks[0], w.waves.len(), pitches.len());
    }
}
// Pitch estimate per block: median autocorrelation pitch over voiced 40 ms frames of all samples.
#[allow(dead_code)]
fn unused() {}

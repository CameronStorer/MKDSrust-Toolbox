import { useEffect, useRef, useState } from 'react';
import { binary, pickFile } from '../backend';
import { useStore } from '../store';

export function AudioWorkspace() {
  const snapshot = useStore((s) => s.snapshot);
  const busy = useStore((s) => s.busy);
  const [kind, setKind] = useState(0);
  const [index, setIndex] = useState(0);
  const [sample, setSample] = useState(0);
  const [sequence, setSequence] = useState(0);
  const [bank, setBank] = useState(0);
  const [source, setSource] = useState('');
  const [route, setRoute] = useState('auto');
  const [rate, setRate] = useState(16000);
  const [encoding, setEncoding] = useState(2);
  const [stereo, setStereo] = useState(false);
  const [start, setStart] = useState('');
  const [end, setEnd] = useState('');
  const [url, setUrl] = useState('');
  const lastUrl = useRef('');
  const selected = snapshot?.selected ?? '';
  const isSdat = snapshot?.files.some((f) => f.path === selected && f.format === 'sdat');
  useEffect(() => {
    if (isSdat) return;
    const file = snapshot?.files.find((f) => f.format === 'sdat');
    if (file) void useStore.getState().run({ kind: 'Select', path: file.path });
  }, [isSdat, snapshot?.files]);
  useEffect(
    () => () => {
      if (lastUrl.current) URL.revokeObjectURL(lastUrl.current);
    },
    [],
  );
  async function play() {
    try {
      const bytes = await binary({
        path: selected,
        mode: 'audio-wav',
        texture: kind,
        bank: index,
        start: sample,
        length: 10,
        palette: null,
        screen: null,
      });
      if (lastUrl.current) URL.revokeObjectURL(lastUrl.current);
      lastUrl.current = URL.createObjectURL(new Blob([bytes], { type: 'audio/wav' }));
      setUrl(lastUrl.current);
    } catch (error) {
      useStore.getState().set({ error: String(error) });
    }
  }
  const resources = snapshot?.audio_resources ?? [];
  const current = resources.find((r) => r.kind === kind && r.index === index);
  return (
    <div className="panel audio-workspace">
      <div className="section-label">SDAT · {selected || 'Open a ROM to browse audio'}</div>
      <div className="audio-columns">
        <div className="audio-tree" role="tree" aria-label="SDAT tree">
          {[
            [0, 'Sequences'],
            [1, 'Sequence archives'],
            [2, 'Instrument banks'],
            [3, 'Wave archives'],
            [7, 'Streams'],
          ].map(([type, name]) => (
            <details key={type} open>
              <summary>{name}</summary>
              {resources
                .filter((r) => r.kind === type)
                .map((r) => (
                  <button
                    role="treeitem"
                    aria-selected={kind === r.kind && index === r.index}
                    key={r.index}
                    className={kind === r.kind && index === r.index ? 'active' : ''}
                    onClick={() => {
                      setKind(r.kind);
                      setIndex(r.index);
                      setSample(0);
                      if (r.kind === 0) setSequence(r.index);
                      if (r.kind === 2) setBank(r.index);
                    }}
                  >
                    {r.index} · {r.name} · {Math.ceil(r.bytes / 1024)} KiB
                  </button>
                ))}
            </details>
          ))}
        </div>
        <div className="audio-editor engine-tools">
          <h2>{current?.name ?? 'Audio preview'}</h2>
          {kind === 3 && (
            <label>
              Sample
              <input
                aria-label="Sample index"
                type="number"
                min={0}
                max={Math.max(0, (current?.samples ?? 1) - 1)}
                value={sample}
                onChange={(e) => setSample(Number(e.target.value))}
              />
            </label>
          )}
          <button disabled={!isSdat || ![0, 3, 7].includes(kind)} onClick={() => void play()}>
            Play preview
          </button>
          {url && <audio key={url} src={url} controls autoPlay aria-label="Audio preview" />}
          <p className="muted">
            Ten-second software preview; basic envelopes and supported sequence commands.
          </p>
          <div className="section-label">IMPORT SONG</div>
          <fieldset disabled={!snapshot?.project_path || busy || !isSdat}>
            <button
              onClick={() =>
                void pickFile(['wav', 'mid', 'midi']).then((p) => {
                  if (p) setSource(p);
                })
              }
            >
              Choose WAV / MIDI…
            </button>
            <p className="source-path">{source || 'Choose your own audio file'}</p>
            <label>
              Route
              <select
                aria-label="Song route"
                value={route}
                onChange={(e) => setRoute(e.target.value)}
              >
                <option value="auto">Automatic</option>
                <option value="sampled">Sampled audio</option>
                <option value="midi">MIDI sequence</option>
              </select>
            </label>
            <label>
              Replace sequence
              <select
                aria-label="Replace sequence"
                value={sequence}
                onChange={(e) => setSequence(Number(e.target.value))}
              >
                {resources
                  .filter((r) => r.kind === 0)
                  .map((r) => (
                    <option value={r.index} key={r.index}>
                      {r.index} · {r.name}
                    </option>
                  ))}
              </select>
            </label>
            <label>
              MIDI bank
              <select
                aria-label="MIDI bank"
                value={bank}
                onChange={(e) => setBank(Number(e.target.value))}
              >
                {resources
                  .filter((r) => r.kind === 2)
                  .map((r) => (
                    <option value={r.index} key={r.index}>
                      {r.index} · {r.name}
                    </option>
                  ))}
              </select>
            </label>
            <label>
              Sample rate
              <select
                aria-label="Song sample rate"
                value={rate}
                onChange={(e) => setRate(Number(e.target.value))}
              >
                {[8000, 11025, 16000, 22050, 32000, 44100].map((v) => (
                  <option key={v} value={v}>
                    {v} Hz
                  </option>
                ))}
              </select>
            </label>
            <label>
              Encoding
              <select
                aria-label="Song encoding"
                value={encoding}
                onChange={(e) => setEncoding(Number(e.target.value))}
              >
                <option value={2}>IMA-ADPCM · smallest</option>
                <option value={1}>PCM16</option>
                <option value={0}>PCM8</option>
              </select>
            </label>
            <label>
              <input
                type="checkbox"
                checked={stereo}
                onChange={(e) => setStereo(e.target.checked)}
              />
              Preserve stereo
            </label>
            <label>
              Loop start (source sample)
              <input
                aria-label="Loop start"
                type="number"
                min={0}
                placeholder="No loop"
                value={start}
                onChange={(e) => setStart(e.target.value)}
              />
            </label>
            <label>
              Loop end / trim (source sample)
              <input
                aria-label="Loop end"
                type="number"
                min={1}
                placeholder="End of file"
                value={end}
                onChange={(e) => setEnd(e.target.value)}
              />
            </label>
            <button
              disabled={!source}
              onClick={() =>
                void useStore
                  .getState()
                  .run({
                    kind: 'ImportSong',
                    path: selected,
                    source,
                    options: {
                      sequence,
                      bank,
                      route,
                      rate,
                      stereo,
                      encoding,
                      loop_start: start === '' ? null : Number(start),
                      loop_end: end === '' ? null : Number(end),
                    },
                  })
              }
            >
              Import song
            </button>
            {kind === 3 && (
              <button
                disabled={!source}
                onClick={() =>
                  void useStore
                    .getState()
                    .run({
                      kind: 'ReplaceEffect',
                      path: selected,
                      source,
                      archive: index,
                      sample,
                      rate,
                      encoding,
                    })
                }
              >
                Replace selected sample
              </button>
            )}
          </fieldset>
          {snapshot?.song_report && (
            <p role="status">
              Imported {snapshot.song_report.route} ·{' '}
              {Math.ceil(snapshot.song_report.ram_bytes / 1024)} KiB RAM
              {snapshot.song_report.loop_sample !== null
                ? ` · aligned loop sample ${snapshot.song_report.loop_sample}`
                : ''}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

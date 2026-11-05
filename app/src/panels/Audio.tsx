import { useEffect, useRef, useState } from 'react';
import { Resizer } from '../Resizer';
import { binary, pickFile } from '../backend';
import { useStore } from '../store';
import type { Snapshot } from '../bindings';

export function AudioWorkspace() {
  const snapshot = useStore((s) => s.snapshot);
  const busy = useStore((s) => s.busy);
  const [kind, setKind] = useState(0);
  const [index, setIndex] = useState(
    snapshot?.wizard_catalog.courses.find((c) => c.id === 20)?.sequence ?? 0,
  );
  const [sample, setSample] = useState(0);
  const [sequence, setSequence] = useState(index);
  const [bank, setBank] = useState(0);
  const [source, setSource] = useState('');
  const [route, setRoute] = useState('auto');
  const [rate, setRate] = useState(16000);
  const [encoding, setEncoding] = useState(2);
  const [stereo, setStereo] = useState(false);
  const [start, setStart] = useState('');
  const [end, setEnd] = useState('');
  const [url, setUrl] = useState('');
  const [replacing, setReplacing] = useState(false);
  const [loading, setLoading] = useState(false);
  const [playing, setPlaying] = useState(false);
  const audio = useRef<HTMLAudioElement>(null);
  const generation = useRef(0);
  const advanced = useStore((s) => s.advanced);
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
  async function play(previewSample = sample) {
    const requestGeneration = generation.current;
    audio.current?.pause();
    setPlaying(false);
    setLoading(true);
    try {
      const bytes = await binary({
        path: selected,
        mode: 'audio-wav',
        texture: kind,
        bank: index,
        start: previewSample,
        length: 0,
        palette: null,
        screen: null,
      });
      if (requestGeneration !== generation.current) return;
      if (lastUrl.current) URL.revokeObjectURL(lastUrl.current);
      lastUrl.current = URL.createObjectURL(new Blob([bytes], { type: 'audio/wav' }));
      setUrl(lastUrl.current);
    } catch (error) {
      useStore.getState().set({ error: String(error) });
    } finally {
      setLoading(false);
    }
  }
  useEffect(() => {
    generation.current += 1;
    audio.current?.pause();
    setPlaying(false);
    setUrl('');
  }, [selected, kind, index]);
  const resources = snapshot?.audio_resources ?? [];
  const current = resources.find((r) => r.kind === kind && r.index === index);
  useEffect(() => {
    if (current?.kind === 0) {
      setBank(current.bank ?? 0);
      setSequence(current.index);
    }
  }, [current?.kind, current?.bank, current?.index]);
  return (
    <div className="panel audio-workspace">
      <div className="section-label">{advanced ? `SDAT · ${selected}` : 'Music & sounds'}</div>
      <div className="audio-columns">
        <div className="audio-tree" role="tree" aria-label="SDAT tree">
          {[
            [0, 'Music & jingles'],
            [1, 'Sound effect sequences'],
            [2, 'Instrument banks'],
            [3, 'Voices & sound effects'],
            [7, 'Streams'],
          ]
            .filter(([type]) => advanced || [0, 3, 7].includes(Number(type)))
            .map(([type, name]) => (
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
                        if (r.kind === 0) {
                          setSequence(r.index);
                          setBank(r.bank ?? 0);
                        }
                        if (r.kind === 2) setBank(r.index);
                      }}
                    >
                      {songName(r.index, r.kind, r.name, snapshot)} · {Math.ceil(r.bytes / 1024)}{' '}
                      KiB
                    </button>
                  ))}
              </details>
            ))}
        </div>
        <Resizer name="audio-tree" min={160} max={700} />
        <div className="audio-editor engine-tools">
          <h2>
            {current
              ? songName(current.index, current.kind, current.name, snapshot)
              : 'Audio preview'}
          </h2>
          {kind === 3 && (
            <div className="sample-list">
              {Array.from({ length: current?.samples ?? 0 }, (_, i) => (
                <button
                  key={i}
                  disabled={loading}
                  onClick={() => {
                    setSample(i);
                    void play(i);
                  }}
                >
                  Play sound {i + 1}
                </button>
              ))}
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
            </div>
          )}
          <button
            disabled={loading || !isSdat || ![0, 3, 7].includes(kind)}
            onClick={() => void play()}
          >
            {loading ? 'Preparing song…' : 'Play preview'}
          </button>
          <button
            disabled={!url}
            onClick={() => {
              audio.current?.pause();
              if (audio.current) audio.current.currentTime = 0;
              setPlaying(false);
            }}
          >
            Stop preview
          </button>
          <span role="status">
            {playing
              ? 'Playing'
              : loading
                ? 'Loading audio'
                : url
                  ? 'Ready to play'
                  : 'Choose a song or sound'}
          </span>
          <audio
            ref={audio}
            src={url || undefined}
            controls
            aria-label="Audio preview"
            onPlay={() => setPlaying(true)}
            onPause={() => setPlaying(false)}
            onError={() =>
              useStore.getState().set({
                error: 'The audio player could not load this preview. Try Play preview again.',
              })
            }
            onLoadedData={() => {
              void audio.current?.play().catch((error) =>
                useStore.getState().set({
                  error: `Playback could not start: ${String(error)}. Press Play on the audio controls.`,
                }),
              );
            }}
          />
          <button disabled={!snapshot?.manifest} onClick={() => setReplacing(true)}>
            {kind === 3 ? 'Replace this sound' : 'Replace this song'}
          </button>
          <p className="muted">
            Full arrangement through its first loop (up to five minutes), using its linked
            instruments. Drag the player position bar to seek.
          </p>
          <details open={advanced || replacing}>
            <summary>Advanced · import settings</summary>
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
                  void useStore.getState().run({
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
                    void useStore.getState().run({
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
          </details>
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
function songName(index: number, kind: number, fallback: string, snapshot: Snapshot | null) {
  if (kind !== 0) return fallback;
  const catalog = snapshot?.wizard_catalog;
  const slots = catalog?.cups.flat() ?? [];
  const courses = catalog?.courses.filter((c) => c.sequence === index) ?? [];
  const labels = courses.map((c) => {
    const position = slots.indexOf(c.id);
    return (
      snapshot?.names.find(
        (n) =>
          n.path.includes('_us.') && n.path.endsWith('common.bmg') && n.index === 45 + position,
      )?.text ?? c.stem.replaceAll('_', ' ')
    );
  });
  return labels.length
    ? labels.join(' / ') + ' · course theme'
    : 'Menu / results / jingle ' + index + ' · ' + fallback;
}

import { useEffect, useState } from 'react';
import { useStore } from '../store';
import { usePreview } from '../hooks';
import { binary } from '../backend';
export function Cells() {
  const advanced = useStore((s) => s.advanced);
  const snapshot = useStore((s) => s.snapshot);
  const path = snapshot?.selected ?? '';
  const metadata = usePreview(path);
  const [sequence, setSequence] = useState(0);
  const [frame, setFrame] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [url, setUrl] = useState('');
  const [error, setError] = useState('');
  const [palette, setPalette] = useState('');
  const [cell, setCell] = useState('');
  useEffect(() => {
    setSequence(0);
    setFrame(0);
    setPlaying(false);
    setCell('');
    setPalette('');
  }, [path]);
  const frames = metadata.sequences[sequence];
  useEffect(() => {
    if (!playing || !frames?.length) return;
    const timer = setTimeout(
      () => setFrame((f) => (f + 1) % frames.length),
      (Math.max(1, frames[frame]?.duration ?? 1) * 1000) / 60,
    );
    return () => clearTimeout(timer);
  }, [playing, frame, frames]);
  useEffect(() => {
    let active = true;
    let object = '';
    void binary({
      path,
      mode: 'cell-png',
      start: frame,
      length: 0,
      palette: palette || null,
      screen: cell || null,
      texture: sequence,
      bank: 0,
    })
      .then((b) => {
        if (active) {
          object = URL.createObjectURL(new Blob([b], { type: 'image/png' }));
          setUrl(object);
          setError('');
        }
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
      if (object) URL.revokeObjectURL(object);
    };
  }, [path, palette, cell, sequence, frame, snapshot]);
  return (
    <div className="panel cell-panel">
      <details open={advanced}>
        <summary>Details - image pairing</summary>
        <div className="source-controls">
          <label>
            Palette
            <select
              aria-label="Cell palette"
              value={palette}
              onChange={(e) => setPalette(e.target.value)}
            >
              <option value="">Auto pair NCLR</option>
              {snapshot?.files
                .filter((f) => f.format === 'nclr')
                .map((f) => (
                  <option key={f.path} value={f.path}>
                    {f.path}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Cell bank
            <select aria-label="Cell bank" value={cell} onChange={(e) => setCell(e.target.value)}>
              <option value="">Auto pair NCER</option>
              {snapshot?.files
                .filter((f) => f.format === 'ncer')
                .map((f) => (
                  <option key={f.path} value={f.path}>
                    {f.path}
                  </option>
                ))}
            </select>
          </label>
        </div>
      </details>
      <div className="scene-toolbar">
        <label>
          {frames ? 'Sequence' : 'Cell'}
          <select
            aria-label="Cell or sequence"
            value={sequence}
            onChange={(e) => {
              setSequence(Number(e.target.value));
              setFrame(0);
              setPlaying(false);
            }}
          >
            {(metadata.sequences.length ? metadata.sequences : metadata.cells).map((_, i) => (
              <option key={i} value={i}>
                {i}
              </option>
            ))}
          </select>
        </label>
        {frames && (
          <>
            <button onClick={() => setPlaying(!playing)}>
              {playing ? 'Pause cells' : 'Play cells'}
            </button>
            <input
              aria-label="Cell animation frame"
              type="range"
              min={0}
              max={Math.max(0, frames.length - 1)}
              value={frame}
              onChange={(e) => {
                setFrame(Number(e.target.value));
                setPlaying(false);
              }}
            />
          </>
        )}
      </div>
      <div className="canvas-area checker">
        {error ? (
          <p role="alert">{error}</p>
        ) : url ? (
          <img className="cell-preview" src={url} alt="Cell composition" />
        ) : (
          'Loading cells…'
        )}
      </div>
      <div className="cell-frames">
        {frames?.map((f, i) => (
          <button
            key={i}
            className={i === frame ? 'active' : ''}
            onClick={() => {
              setFrame(i);
              setPlaying(false);
            }}
          >
            Frame {i} · Cell {f.cell} · {f.duration} ticks
          </button>
        ))}
      </div>
      <p className="hint">
        NCER composition with NCGR + NCLR. NANR translation, scale and rotation preview at 60 ticks
        per second.
      </p>
    </div>
  );
}

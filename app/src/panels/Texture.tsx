import { Cells } from './Cells';
import { useEffect, useRef, useState } from 'react';
import {
  Grid2X2,
  Minus,
  Plus,
  Image as ImageIcon,
  Move,
  Maximize,
  Palette as PaletteIcon,
} from 'lucide-react';
import { binary } from '../backend';
import { useStore } from '../store';
import { useView } from '../view';
import { companion, usePreview } from '../hooks';
import { hex555, rgb555, fromHex, shift555 } from '../colors';
import { PaintTools } from './PaintTools';
export function Texture() {
  const format = useStore(
    (s) => s.snapshot?.files.find((f) => f.path === s.snapshot?.selected)?.format,
  );
  return format === 'ncer' || format === 'nanr' ? <Cells /> : <RasterTexture />;
}
function RasterTexture() {
  const snapshot = useStore((s) => s.snapshot);
  const view = useView();
  const advanced = useStore((s) => s.advanced);
  const [url, setUrl] = useState('');
  const [error, setError] = useState('');
  const [dimensions, setDimensions] = useState([0, 0]);
  const canvas = useRef<HTMLCanvasElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const files = snapshot?.files ?? [];
  const selected = snapshot?.selected ?? '';
  const file = files.find((f) => f.path === selected);
  const path =
    view.tiles ||
    (file?.format === 'nclr' ? companion(files, selected, 'ncgr') || selected : selected);
  const palette =
    view.palette || (file?.format === 'nclr' ? selected : companion(files, path, 'nclr'));
  const screen = view.screen;
  const metadata = usePreview(path);
  const image = useRef<HTMLImageElement | null>(null);
  useEffect(() => {
    if (!path) {
      setUrl('');
      return;
    }
    let active = true;
    let object = '';
    setError('');
    void binary({
      path,
      mode: 'png',
      palette: palette || null,
      screen: screen || null,
      texture: view.texture,
      bank: view.bank,
      start: 0,
      length: 0,
    })
      .then((b) => {
        if (active) {
          object = URL.createObjectURL(new Blob([b], { type: 'image/png' }));
          setUrl(object);
        }
      })
      .catch((e) => {
        if (active) {
          setError(String(e));
          setUrl('');
        }
      });
    return () => {
      active = false;
      if (object) URL.revokeObjectURL(object);
    };
  }, [path, palette, screen, view.texture, view.bank, snapshot]);
  useEffect(() => {
    if (!url) return;
    const img = new window.Image();
    img.onload = () => {
      image.current = img;
      setDimensions([img.width, img.height]);
      const c = canvas.current;
      if (c) {
        c.width = img.width;
        c.height = img.height;
        const ctx = c.getContext('2d');
        if (ctx) {
          ctx.imageSmoothingEnabled = false;
          ctx.drawImage(img, 0, 0);
        }
      }
    };
    img.src = url;
    return () => {
      img.onload = null;
    };
  }, [url]);
  return (
    <div className="panel texture">
      <div className="toolbar texture-toolbar">
        <ImageIcon size={14} />
        <span>
          {!advanced
            ? (snapshot?.game_entries.find((e) => e.path === selected)?.label ?? 'Image preview')
            : (file?.name ?? 'Image preview')}
        </span>
        <div className="grow" />
        <button title="Zoom out" onClick={() => view.set({ zoom: Math.max(0.5, view.zoom - 0.5) })}>
          <Minus size={14} />
        </button>
        <span className="mono zoom-label">{Math.round(view.zoom * 100)}%</span>
        <button title="Zoom in" onClick={() => view.set({ zoom: Math.min(16, view.zoom + 0.5) })}>
          <Plus size={14} />
        </button>
        <button
          title="Fit view"
          disabled={!dimensions[0] || !dimensions[1]}
          onClick={() =>
            view.set({
              zoom: Math.min(
                16,
                Math.max(
                  0.5,
                  Math.floor(
                    Math.min(
                      (scroller.current?.clientWidth ?? 512) / dimensions[0],
                      (scroller.current?.clientHeight ?? 512) / dimensions[1],
                    ) * 2,
                  ) / 2,
                ),
              ),
            })
          }
        >
          <Maximize size={14} />
        </button>
        <button
          title="Pixel grid"
          className={view.grid ? 'active' : ''}
          onClick={() => view.set({ grid: !view.grid })}
        >
          <Grid2X2 size={14} />
        </button>
        <button
          title="Tile grid"
          aria-pressed={view.tileGrid}
          onClick={() => view.set({ tileGrid: !view.tileGrid })}
        >
          8×8
        </button>
      </div>
      <details className="format-details" open={advanced}>
        <summary>Details · file pairing</summary>
        <div className="source-controls">
          <label>
            Tiles
            <select
              aria-label="Tile source"
              value={view.tiles}
              onChange={(e) => view.set({ tiles: e.target.value })}
            >
              <option value="">Follow selection</option>
              {files
                .filter((f) => ['ncgr', 'nsbtx', 'nsbmd'].includes(f.format))
                .map((f) => (
                  <option key={f.path} value={f.path}>
                    {f.name}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Palette
            <select
              aria-label="Palette source"
              value={view.palette}
              onChange={(e) => view.set({ palette: e.target.value, bank: 0 })}
            >
              <option value="">Auto pair</option>
              {files
                .filter((f) => f.format === 'nclr')
                .map((f) => (
                  <option key={f.path} value={f.path}>
                    {f.name}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Screen
            <select
              aria-label="Screen source"
              value={screen}
              onChange={(e) => view.set({ screen: e.target.value })}
            >
              <option value="">Raw tiles</option>
              {files
                .filter((f) => f.format === 'nscr')
                .map((f) => (
                  <option key={f.path} value={f.path}>
                    {f.name}
                  </option>
                ))}
            </select>
          </label>
        </div>
        {metadata.textures.length > 0 && (
          <div className="source-controls">
            <label>
              Texture
              <select
                value={view.texture}
                onChange={(e) => view.set({ texture: Number(e.target.value) })}
              >
                {metadata.textures.map((name, i) => (
                  <option key={i} value={i}>
                    {name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Texture palette
              <select
                value={view.bank}
                onChange={(e) => view.set({ bank: Number(e.target.value) })}
              >
                {metadata.palettes.map((name, i) => (
                  <option key={i} value={i}>
                    {name}
                  </option>
                ))}
              </select>
            </label>
          </div>
        )}
      </details>
      {screen && <button onClick={() => view.set({ screen: '' })}>Paint individual tiles</button>}
      <PaintTools canvas={canvas} path={path} palette={palette} screen={screen} ready={url} />
      <div
        className="canvas-area checker"
        ref={scroller}
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          const el = e.currentTarget;
          const x = e.clientX,
            y = e.clientY,
            left = el.scrollLeft,
            top = el.scrollTop;
          el.setPointerCapture(e.pointerId);
          const move = (ev: PointerEvent) => {
            el.scrollLeft = left - (ev.clientX - x);
            el.scrollTop = top - (ev.clientY - y);
          };
          const end = () => {
            el.removeEventListener('pointermove', move);
            el.removeEventListener('pointerup', end);
            el.removeEventListener('pointercancel', end);
          };
          el.addEventListener('pointermove', move);
          el.addEventListener('pointerup', end);
          el.addEventListener('pointercancel', end);
        }}
        onWheel={(e) => {
          if (e.ctrlKey) {
            e.preventDefault();
            view.set({
              zoom: Math.max(0.5, Math.min(16, view.zoom + (e.deltaY < 0 ? 0.5 : -0.5))),
            });
          }
        }}
      >
        {url ? (
          <div
            className="canvas-wrap"
            style={{ width: dimensions[0] * view.zoom, height: dimensions[1] * view.zoom }}
          >
            <canvas
              aria-label="Texture preview"
              ref={canvas}
              style={{ width: '100%', height: '100%' }}
            />
            {view.grid && view.zoom >= 3 && (
              <div
                className="pixel-grid"
                style={{ backgroundSize: `${view.zoom}px ${view.zoom}px` }}
              />
            )}
            {view.tileGrid && (
              <div
                className="pixel-grid tile-grid"
                style={{ backgroundSize: `${view.zoom * 8}px ${view.zoom * 8}px` }}
              />
            )}
          </div>
        ) : (
          <div className="empty">
            <ImageIcon size={32} />
            <p>{path ? error || 'Loading preview…' : 'Select tiles, a palette or a texture.'}</p>
          </div>
        )}
      </div>
      <div className="panel-foot">
        <span className="mono">
          {dimensions[0]} × {dimensions[1]} px
        </span>
        <span>
          <Move size={11} /> Drag to pan · Ctrl scroll to zoom
        </span>
      </div>
    </div>
  );
}
export function Palette() {
  const snapshot = useStore((s) => s.snapshot);
  const run = useStore((s) => s.run);
  const view = useView();
  const files = snapshot?.files ?? [];
  const selected = snapshot?.selected ?? '';
  const file = files.find((f) => f.path === selected);
  const tx = ['nsbtx', 'nsbmd'].includes(file?.format ?? '');
  const path = tx
    ? selected
    : view.palette || (file?.format === 'nclr' ? selected : companion(files, selected, 'nclr'));
  const metadata = usePreview(path);
  const [bankColors, setBankColors] = useState<number[]>([]);
  useEffect(() => {
    if (!tx) return;
    let alive = true;
    void binary({
      path,
      mode: 'palette-colors',
      texture: view.texture,
      bank: view.bank,
      palette: null,
      screen: null,
      start: 0,
      length: 0,
    })
      .then((b) => {
        if (alive) setBankColors(Array.from(new Uint16Array(b)));
      })
      .catch((e) => useStore.getState().set({ error: String(e) }));
    return () => {
      alive = false;
    };
  }, [path, tx, view.texture, view.bank, snapshot]);
  const preview = tx ? { ...metadata, colors: bankColors } : metadata;
  const [anchor, setAnchor] = useState(0);
  const [hsv, setHsv] = useState([0, 0, 0]);
  useEffect(() => {
    useView.setState({ selectedColors: [0], bank: 0 });
    setAnchor(0);
    setHsv([0, 0, 0]);
  }, [path]);
  const index = Math.min(view.selectedColors[0] ?? 0, Math.max(0, preview.colors.length - 1));
  const color = preview.colors[index] ?? 0;
  const rgb = rgb555(color);
  const apply = (indices: number[], colors: number[]) =>
    void run(
      tx
        ? { kind: 'TexturePalette', path, texture: view.texture, bank: view.bank, indices, colors }
        : { kind: 'Palette', path, indices, colors },
    );
  return (
    <div className="panel palette-panel">
      <div className="toolbar">
        <PaletteIcon size={14} />
        <span>{preview.colors.length} COLOURS</span>
        <div className="grow" />
        <label>
          Bank{' '}
          <input
            aria-label="Palette bank"
            type="number"
            min="0"
            max={
              tx
                ? Math.max(0, metadata.palettes.length - 1)
                : Math.max(0, Math.ceil(preview.colors.length / (preview.palette_stride || 16)) - 1)
            }
            value={view.bank}
            onChange={(e) => view.set({ bank: Math.max(0, Number(e.target.value)) })}
          />
        </label>
      </div>
      {preview.colors.length > 0 ? (
        <div className="palette-content">
          <p className="hint">Click a colour to edit. Shift click selects a range.</p>
          <div className="swatches">
            {preview.colors.map((c, i) => (
              <button
                key={i}
                title={`Colour ${i} · ${hex555(c)} · BGR555 0x${c.toString(16).padStart(4, '0')}`}
                aria-label={`Colour ${i}`}
                aria-pressed={view.selectedColors.includes(i)}
                style={{ background: hex555(c) }}
                onClick={(e) => {
                  if (e.shiftKey) {
                    const a = Math.min(anchor, i),
                      b = Math.max(anchor, i);
                    view.set({
                      selectedColors: Array.from({ length: b - a + 1 }, (_, n) => a + n),
                    });
                  } else {
                    setAnchor(i);
                    view.set({ selectedColors: [i] });
                  }
                }}
              />
            ))}
          </div>
          <div className="color-editor">
            <div className="color-sample" style={{ background: hex555(color) }} />
            <div>
              <strong>Colour {index.toString().padStart(3, '0')}</strong>
              <div className="mono muted">
                0x{color.toString(16).padStart(4, '0').toUpperCase()} · BGR555
              </div>
            </div>
            <input
              aria-label="Colour picker"
              type="color"
              value={hex555(color)}
              disabled={!snapshot?.manifest}
              onChange={(e) => apply([index], [fromHex(e.target.value)])}
            />
          </div>
          <div className="rgb-edit">
            {['R', 'G', 'B'].map((channel, i) => (
              <label key={channel}>
                {channel}
                <input
                  aria-label={`${channel} channel`}
                  type="range"
                  min="0"
                  max="31"
                  disabled={!snapshot?.manifest}
                  value={rgb[i]}
                  onChange={(e) => {
                    const channels = [...rgb];
                    channels[i] = Number(e.target.value);
                    apply([index], [channels[0] | (channels[1] << 5) | (channels[2] << 10)]);
                  }}
                />
                <span className="mono">{rgb[i]}</span>
              </label>
            ))}
          </div>
          <div className="color-values mono">
            {hex555(color).toUpperCase()}{' '}
            <span>{rgb.map((c) => (c << 3) | (c >> 2)).join(', ')}</span>
          </div>
          <div className="section-label">
            ADJUST SELECTED RANGE <span>{view.selectedColors.length}</span>
          </div>
          {['Hue', 'Saturation', 'Brightness'].map((title, i) => (
            <label className="hsv" key={title}>
              {title}
              <input
                type="range"
                min={i === 0 ? -180 : -100}
                max={i === 0 ? 180 : 100}
                value={hsv[i]}
                onChange={(e) =>
                  setHsv((old) => old.map((v, n) => (n === i ? Number(e.target.value) : v)))
                }
              />
              <span className="mono">
                {hsv[i]}
                {i === 0 ? '°' : '%'}
              </span>
            </label>
          ))}
          <button
            className="wide"
            disabled={!snapshot?.manifest}
            onClick={() => {
              const indices = view.selectedColors.filter((i) => i < preview.colors.length);
              apply(
                indices,
                indices.map((i) => shift555(preview.colors[i], hsv[0], hsv[1], hsv[2])),
              );
              setHsv([0, 0, 0]);
            }}
          >
            Apply adjustment
          </button>
        </div>
      ) : (
        <div className="empty">
          <PaletteIcon size={24} />
          <p>
            Select an NCLR palette.
            <br />
            Pair it with tiles in the preview.
          </p>
        </div>
      )}
    </div>
  );
}

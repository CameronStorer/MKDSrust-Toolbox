import { useEffect, useRef, useState, type RefObject } from 'react';
import { binary, pickFile, savePath } from '../backend';
import { useStore } from '../store';
import { useView } from '../view';
import { fromHex, hex555 } from '../colors';
import { stroke, type Point, type Tool } from '../paint';
import { bytes } from './Properties';
interface Props {
  canvas: RefObject<HTMLCanvasElement>;
  path: string;
  palette: string;
  screen: string;
  ready: string;
}
export function PaintTools({ canvas, path, palette, screen, ready }: Props) {
  const { snapshot, run } = useStore();
  const view = useView();
  const [direct, setDirect] = useState(false);
  const [tool, setTool] = useState<Tool>('pan');
  const [pixels, setPixels] = useState<number[]>([]);
  const [colors, setColors] = useState<number[]>([]);
  const [color, setColor] = useState(0);
  const [width, setWidth] = useState(0);
  const [height, setHeight] = useState(0);
  const [source, setSource] = useState('');
  const [format, setFormat] = useState(3);
  const [dither, setDither] = useState(false);
  const [preview, setPreview] = useState('');
  const [recipe, setRecipe] = useState('');
  useEffect(() => {
    setSource('');
    selection.current = null;
    void run({ kind: 'CancelGraphic' });
  }, [path, run]);
  const selection = useRef<[Point, Point] | null>(null);
  const tx = snapshot?.files.find((f) => f.path === path)?.format === 'nsbtx';
  const editable =
    !!snapshot?.manifest &&
    !screen &&
    ['ncgr', 'nsbtx'].includes(snapshot?.files.find((f) => f.path === path)?.format ?? '');
  useEffect(() => {
    if (!editable) return;
    let active = true;
    const request = {
      path,
      mode: 'paint-data',
      start: 0,
      length: 0,
      palette: null,
      screen: null,
      texture: view.texture,
      bank: view.bank,
    };
    void binary(request)
      .then((b) => {
        if (!active) return;
        const d = new DataView(b);
        setWidth(d.getUint32(0, true));
        setHeight(d.getUint32(4, true));
        setDirect(d.getUint32(8, true) === 7);
        setPixels(Array.from(new Uint16Array(b.slice(12))));
      })
      .catch(() => {
        setPixels([]);
      });
    void binary({ ...request, path: tx ? path : palette, mode: 'palette-colors' })
      .then((b) => {
        if (active) setColors(Array.from(new Uint16Array(b)));
      })
      .catch(() => setColors([]));
    return () => {
      active = false;
    };
  }, [editable, path, palette, tx, view.texture, view.bank, snapshot]);

  useEffect(() => {
    const c = canvas.current;
    if (!c || !editable || !pixels.length || tool === 'pan') return;
    const at = (e: PointerEvent): Point => {
      const r = c.getBoundingClientRect();
      return [
        Math.max(0, Math.min(width - 1, Math.floor(((e.clientX - r.left) / r.width) * width))),
        Math.max(0, Math.min(height - 1, Math.floor(((e.clientY - r.top) / r.height) * height))),
      ];
    };
    const down = (e: PointerEvent) => {
      if (e.button !== 0) return;
      e.stopPropagation();
      e.preventDefault();
      c.setPointerCapture(e.pointerId);
      const a = at(e);
      let last = a;
      const changes = new Map<number, [number, number, number]>();
      const before = c.getContext('2d')?.getImageData(0, 0, width, height);
      if (tool === 'eyedropper') {
        setColor(pixels[a[1] * width + a[0]]);
        return;
      }
      const paint = (points: [number, number, number][]) => {
        const ctx = c.getContext('2d');
        for (const [x, y, v] of points) {
          changes.set(y * width + x, [x, y, v]);
          if (ctx) {
            ctx.fillStyle = hex555(direct ? v : (colors[v] ?? 0));
            ctx.fillRect(x, y, 1, 1);
          }
        }
      };
      if (tool === 'pencil' || tool === 'fill')
        paint(stroke(tool, pixels, width, height, a, a, color));
      const move = (ev: PointerEvent) => {
        const b = at(ev);
        if (tool === 'pencil') {
          paint(stroke(tool, pixels, width, height, last, b, color));
          last = b;
        } else if (tool === 'line' || tool === 'rectangle') {
          if (before) c.getContext('2d')?.putImageData(before, 0, 0);
          changes.clear();
          paint(stroke(tool, pixels, width, height, a, b, color));
        }
      };
      const end = (ev: PointerEvent) => {
        const b = at(ev);
        if (tool === 'select') {
          selection.current = [a, b];
          const ctx = c.getContext('2d');
          if (ctx) {
            ctx.strokeStyle = '#ffffff';
            ctx.lineWidth = 1;
            ctx.setLineDash([2, 2]);
            ctx.strokeRect(
              Math.min(a[0], b[0]) + 0.5,
              Math.min(a[1], b[1]) + 0.5,
              Math.abs(a[0] - b[0]) + 1,
              Math.abs(a[1] - b[1]) + 1,
            );
            ctx.setLineDash([]);
          }
        } else if (tool === 'move' && selection.current) {
          const [p, q] = selection.current;
          const dx = b[0] - a[0],
            dy = b[1] - a[1];
          for (let y = Math.min(p[1], q[1]); y <= Math.max(p[1], q[1]); y++)
            for (let x = Math.min(p[0], q[0]); x <= Math.max(p[0], q[0]); x++) paint([[x, y, 0]]);
          for (let y = Math.min(p[1], q[1]); y <= Math.max(p[1], q[1]); y++)
            for (let x = Math.min(p[0], q[0]); x <= Math.max(p[0], q[0]); x++)
              if (x + dx >= 0 && y + dy >= 0 && x + dx < width && y + dy < height)
                paint([[x + dx, y + dy, pixels[y * width + x]]]);
          selection.current = null;
        } else if (tool === 'line' || tool === 'rectangle')
          paint(stroke(tool, pixels, width, height, a, b, color));
        c.removeEventListener('pointermove', move);
        c.removeEventListener('pointerup', end);
        c.removeEventListener('pointercancel', cancel);
        if (changes.size)
          void run({
            kind: 'Paint',
            path,
            palette: palette || null,
            texture: view.texture,
            bank: view.bank,
            points: [...changes.values()],
          }).then(() => {
            if (useStore.getState().error && before) c.getContext('2d')?.putImageData(before, 0, 0);
          });
      };
      const cancel = () => {
        if (before) c.getContext('2d')?.putImageData(before, 0, 0);
        c.removeEventListener('pointermove', move);
        c.removeEventListener('pointerup', end);
        c.removeEventListener('pointercancel', cancel);
      };
      c.addEventListener('pointermove', move);
      c.addEventListener('pointerup', end);
      c.addEventListener('pointercancel', cancel);
    };
    c.addEventListener('pointerdown', down);
    return () => c.removeEventListener('pointerdown', down);
  }, [
    canvas,
    ready,
    editable,
    pixels,
    tool,
    width,
    height,
    color,
    colors,
    direct,
    path,
    palette,
    view.texture,
    view.bank,
    run,
  ]);
  useEffect(() => {
    if (!snapshot?.graphic_import) {
      setPreview('');
      return;
    }
    let url = '';
    let active = true;
    void binary({
      path,
      mode: 'import-preview',
      palette: null,
      screen: null,
      texture: 0,
      bank: 0,
      start: 0,
      length: 0,
    }).then((b) => {
      if (active) {
        url = URL.createObjectURL(new Blob([b], { type: 'image/png' }));
        setPreview(url);
      }
    });
    return () => {
      active = false;
      if (url) URL.revokeObjectURL(url);
    };
  }, [snapshot?.graphic_import, path]);
  return (
    <>
      <div className="paint-toolbar" role="toolbar" aria-label="Painting tools">
        {(
          ['pan', 'pencil', 'fill', 'line', 'rectangle', 'eyedropper', 'select', 'move'] as Tool[]
        ).map((t) => (
          <button
            key={t}
            disabled={t !== 'pan' && (!editable || !pixels.length)}
            aria-label={`Tool ${t}`}
            aria-pressed={tool === t}
            className={tool === t ? 'active' : ''}
            onClick={() => setTool(t)}
          >
            {t}
          </button>
        ))}
        <label>
          Ink{' '}
          {direct ? (
            <input
              aria-label="Paint colour"
              type="color"
              value={hex555(color)}
              onChange={(e) => setColor(fromHex(e.target.value))}
            />
          ) : (
            <input
              aria-label="Paint palette index"
              type="number"
              min={0}
              max={Math.max(0, colors.length - 1)}
              value={color}
              onChange={(e) => setColor(Number(e.target.value))}
            />
          )}
        </label>
        <button
          disabled={!editable}
          onClick={() =>
            void pickFile(['png']).then((p) => {
              if (p) setSource(p);
            })
          }
        >
          Import PNG…
        </button>
        <button
          disabled={!path}
          onClick={() =>
            void savePath('graphic.png').then((output) => {
              if (output)
                void run({
                  kind: 'Export',
                  path,
                  output,
                  to: 'png',
                  palette: palette || null,
                  screen: screen || null,
                  texture: view.texture,
                  bank: view.bank,
                });
            })
          }
        >
          Export PNG…
        </button>
        <button onClick={() => setRecipe(recipe ? '' : 'Recolour a menu')}>? Recipes</button>
      </div>
      {colors.length > 0 && editable && (
        <div className="paint-swatches">
          {colors.slice(0, 256).map((c, i) => (
            <button
              aria-label={`Ink ${i}`}
              aria-pressed={color === i}
              key={i}
              title={`Index ${i}`}
              style={{ background: hex555(c) }}
              onClick={() => setColor(i)}
            />
          ))}
        </div>
      )}
      {tool === 'select' && (
        <p className="hint">Drag a rectangle, then choose Move and drag it to a new position.</p>
      )}
      {screen && (
        <p className="hint">
          Choose Raw tiles to paint. Screen previews retain tile reuse and flips.
        </p>
      )}
      {recipe && (
        <div className="recipe-panel">
          <select aria-label="Recipe" value={recipe} onChange={(e) => setRecipe(e.target.value)}>
            {['Recolour a menu', 'Replace an item icon', 'Replace a kart texture'].map((r) => (
              <option key={r}>{r}</option>
            ))}
          </select>
          <ol>
            {(recipe === 'Recolour a menu'
              ? [
                  'Choose the menu NCGR and matching NCLR in Explorer.',
                  'Select palette entries and adjust their colours, or paint with an ink index.',
                  'Export for DS and test the menu in your emulator.',
                ]
              : recipe === 'Replace an item icon'
                ? [
                    'Open the item NCGR with its companion palette. Choose Raw tiles.',
                    'Import an authored PNG at the existing dimensions. Preview quantization and apply.',
                    'View its cell composition, then export and test item pickup.',
                  ]
                : [
                    'Open the kart NSBTX and select the texture and palette.',
                    'Import your PNG; choose a DS format and inspect the conversion and VRAM size.',
                    'Apply, inspect the kart in Model, then Export for DS and race.',
                  ]
            ).map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
        </div>
      )}
      {source && (
        <div className="import-controls">
          <strong>PNG import · {source}</strong>
          <label>
            DS texture format
            <select
              aria-label="Import texture format"
              disabled={!tx}
              value={format}
              onChange={(e) => {
                setFormat(Number(e.target.value));
                void run({ kind: 'CancelGraphic' });
              }}
            >
              {[
                [2, '4-colour'],
                [3, '16-colour'],
                [4, '256-colour'],
                [1, 'a3i5'],
                [6, 'a5i3'],
                [7, 'Direct'],
              ].map(([n, name]) => (
                <option key={n} value={n}>
                  {name}
                </option>
              ))}
            </select>
          </label>
          <label>
            <input
              type="checkbox"
              checked={dither}
              onChange={(e) => {
                setDither(e.target.checked);
                void run({ kind: 'CancelGraphic' });
              }}
            />
            Dither
          </label>
          <button
            onClick={() =>
              void run({
                kind: 'ImportGraphic',
                path,
                source,
                palette: palette || null,
                texture: view.texture,
                bank: view.bank,
                format,
                dither,
              })
            }
          >
            Preview conversion
          </button>
          {snapshot?.graphic_import && (
            <>
              <p>
                {snapshot.graphic_import.width} × {snapshot.graphic_import.height} ·{' '}
                {snapshot.graphic_import.colors} colours ·{' '}
                {bytes(snapshot.graphic_import.texture_bytes)} / 512 KiB texture VRAM ·{' '}
                {bytes(snapshot.graphic_import.palette_bytes)} / 96 KiB palette VRAM
              </p>
              <div className="import-previews">
                <figure>
                  <figcaption>Before</figcaption>
                  {ready && <img src={ready} alt="Before import" />}
                </figure>
                <figure>
                  <figcaption>After</figcaption>
                  {preview && <img src={preview} alt="After conversion" />}
                </figure>
              </div>
              <button
                className="accent"
                onClick={() =>
                  void run({ kind: 'ApplyGraphic' }).then(() => {
                    if (!useStore.getState().error) setSource('');
                  })
                }
              >
                Apply PNG import
              </button>
            </>
          )}
          <button
            onClick={() => {
              setSource('');
              void run({ kind: 'CancelGraphic' });
            }}
          >
            Cancel import
          </button>
        </div>
      )}
    </>
  );
}

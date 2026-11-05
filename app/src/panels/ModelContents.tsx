import { useEffect, useState } from 'react';
import { binary, pickFile, savePath } from '../backend';
import { useScene } from '../scene-view';
import { useStore } from '../store';
import { selectPart } from '../model-actions';
import { FileLink } from '../files';
import { fromHex, hex555 } from '../colors';

export function LinkedCopies() {
  const { info, linked, set } = useScene();
  if (!info?.model_path) return null;
  return (
    <div className="linked-copies" aria-label="Linked copies">
      <strong>Linked copies</strong>
      <label>
        <input
          type="checkbox"
          checked={linked}
          onChange={(e) => set({ linked: e.target.checked })}
        />
        Apply to all copies
      </label>
      {info.linked_copies.length ? (
        info.linked_copies.map((c) => (
          <span key={c.path} title="Budget measured from this copy in the clean ROM">
            <FileLink path={c.path}>{c.role}</FileLink> · {c.triangles} / {c.budget} triangles
          </span>
        ))
      ) : (
        <span>This model has no linked runtime variants.</span>
      )}
    </div>
  );
}
export function ModelContents() {
  const { info, parts, highlight, bone, material, materialPopup, set } = useScene();
  const detail = info?.bone_details[bone];
  const chain: string[] = [];
  for (let b = detail; b; b = b.parent === null ? undefined : info?.bone_details[b.parent])
    chain.unshift(b.name);
  return (
    <>
      <details open>
        <summary>Parts · {info?.parts.length ?? 0}</summary>
        {info?.parts.map((p, i) => (
          <button
            key={i}
            className={`outliner-entry ${parts.includes(i) ? 'selected' : ''}`}
            aria-label={`Part ${i}: ${p.name}`}
            aria-pressed={parts.includes(i)}
            onClick={(e) => selectPart(i, e.shiftKey || e.ctrlKey || e.metaKey)}
          >
            {p.name} · {p.triangles} tris
          </button>
        ))}
      </details>
      <details open>
        <summary>Bones · {info?.bones.length ?? 0}</summary>
        {info?.bones.map((b, i) => (
          <button
            className={`outliner-entry ${highlight === 'bone' && bone === i ? 'selected' : ''}`}
            key={i}
            aria-label={`Bone ${i}: ${b}`}
            onClick={() =>
              set({ bone: i, highlight: 'bone', playing: false, animation: -1, frame: 0 })
            }
          >
            {i} · {b}
          </button>
        ))}
        {highlight === 'bone' && detail && (
          <div className="inset" aria-label="Bone details">
            <p>{chain.join(' → ')}</p>
            <p>{detail.vertices} assigned vertices</p>
            <p>Clips: {detail.clips.join(', ') || 'No joint tracks'}</p>
            <button
              onClick={() => {
                set({ rigBrush: true, playing: false });
                useStore.getState().set({ workspace: 'Rig & Animate' });
              }}
            >
              Paint vertices to this bone
            </button>
          </div>
        )}
      </details>
      <details open>
        <summary>Materials and textures · {info?.materials.length ?? 0}</summary>
        {info?.materials.map((m, i) => (
          <button
            className={`outliner-entry ${highlight === 'material' && material === i ? 'selected' : ''}`}
            key={i}
            onClick={() =>
              set({
                material: i,
                materialPopup: true,
                highlight: 'material',
                playing: false,
                animation: -1,
              })
            }
          >
            {m}
          </button>
        ))}
      </details>
      {materialPopup && <MaterialPopup />}
    </>
  );
}
function MaterialPopup() {
  const { info, material, set } = useScene();
  const detail = info?.material_details[material];
  const snapshot = useStore((s) => s.snapshot);
  const busy = useStore((s) => s.busy);
  const [url, setUrl] = useState('');
  const [zoom, setZoom] = useState(4);
  const [color, setColor] = useState('#ffffff');
  const [index, setIndex] = useState(0);
  const [error, setError] = useState('');
  useEffect(() => {
    setUrl('');
    if (!detail?.path) return;
    let active = true,
      url = '';
    void binary({
      path: detail.path,
      mode: 'png',
      palette: null,
      screen: null,
      texture: detail.texture_index,
      bank: detail.palette_index,
      start: 0,
      length: 0,
    })
      .then((b) => {
        if (active) {
          url = URL.createObjectURL(new Blob([b], { type: 'image/png' }));
          setUrl(url);
        }
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
      if (url) URL.revokeObjectURL(url);
    };
  }, [detail]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        set({ materialPopup: false });
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [set]);
  if (!detail || !info?.model_path) return null;
  const apply = (
    action:
      | { kind: 'Replace'; source: string }
      | { kind: 'Recolour'; index: number; color: number }
      | { kind: 'Tint'; color: number },
  ) => useStore.getState().run({ kind: 'ModelMaterial', path: info.model_path!, material, action });
  return (
    <div className="modal-backdrop">
      <section
        className="modal project-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Material and texture"
      >
        <div className="modal-header">
          <strong>{detail.name}</strong>
          <div className="grow" />
          <button autoFocus onClick={() => set({ materialPopup: false })}>
            Close material
          </button>
        </div>
        <div className="dialog-content">
          <p>
            Texture: {detail.texture ?? 'None'} · Palette:{' '}
            {detail.palette ?? 'Direct colour / none'}
          </p>
          <button
            onClick={() => {
              set({ highlight: 'material', materialPopup: false });
            }}
          >
            Highlight on model
          </button>
          {detail.path ? (
            <>
              <FileLink path={detail.path} />
              <label>
                Texture zoom
                <input
                  aria-label="Texture zoom"
                  type="range"
                  min="1"
                  max="16"
                  value={zoom}
                  onChange={(e) => setZoom(Number(e.target.value))}
                />
              </label>
              <div className="material-image">
                {url && (
                  <img
                    alt="Bound texture"
                    src={url}
                    style={{
                      transform: `scale(${zoom})`,
                      transformOrigin: 'top left',
                      imageRendering: 'pixelated',
                    }}
                  />
                )}
              </div>
              <div className="material-palette" aria-label="Bound palette">
                {detail.colors.map((c, i) => (
                  <button
                    key={i}
                    aria-label={`Palette colour ${i}`}
                    aria-pressed={index === i}
                    style={{ background: hex555(c) }}
                    onClick={() => {
                      setIndex(i);
                      setColor(hex555(c));
                    }}
                  />
                ))}
              </div>
              <p className="muted">
                Palette edits affect every texture that shares this palette. PNGs are resized to DS
                powers of two and converted to the current format (4×4 compressed becomes indexed
                16-colour).
              </p>
              <fieldset disabled={!snapshot?.project_path || busy}>
                <button
                  onClick={() => {
                    void pickFile(['png']).then((source) => {
                      if (source) return apply({ kind: 'Replace', source });
                    });
                  }}
                >
                  Replace image…
                </button>
                <input
                  aria-label="Recolour value"
                  type="color"
                  value={color}
                  onChange={(e) => setColor(e.target.value)}
                />
                <button
                  onClick={() =>
                    void apply(
                      detail.colors.length
                        ? { kind: 'Recolour', index, color: fromHex(color) }
                        : { kind: 'Tint', color: fromHex(color) },
                    )
                  }
                >
                  Recolour
                </button>
              </fieldset>
              <button
                onClick={() => {
                  void savePath(`${detail.texture}.png`).then((output) => {
                    if (output)
                      return useStore.getState().run({
                        kind: 'Export',
                        path: detail.path!,
                        output,
                        to: 'png',
                        palette: null,
                        screen: null,
                        texture: detail.texture_index,
                        bank: detail.palette_index,
                      });
                  });
                }}
              >
                Export PNG…
              </button>
            </>
          ) : (
            <p>This material has no bound image.</p>
          )}
          {error && <p role="alert">{error}</p>}
        </div>
      </section>
    </div>
  );
}

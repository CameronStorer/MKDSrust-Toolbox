import { useScene } from '../scene-view';
import { useStore } from '../store';

export function modelCommand(command: string) {
  const canvas = document.querySelector<HTMLCanvasElement>('canvas[aria-label="3D viewport"]');
  canvas?.focus();
  canvas?.dispatchEvent(new CustomEvent('model-command', { detail: command }));
}
export function ModelEditBar() {
  const s = useScene();
  const busy = useStore((s) => s.busy);
  const writable = useStore((s) => !!s.snapshot?.project_path);
  const tris = s.liveTriangles ?? s.info?.budget.polygons ?? 0;
  return (
    <div className="model-edit-bar">
      <button
        aria-pressed={s.editMode}
        disabled={!writable || busy}
        onClick={() => modelCommand('tab')}
      >
        {s.editMode ? 'Edit mode · Tab' : 'Object mode · Tab'}
      </button>
      {s.editMode &&
        (['vertex', 'edge', 'face'] as const).map((m, i) => (
          <button
            key={m}
            aria-pressed={s.elementMode === m}
            onClick={() => modelCommand(String(i + 1))}
          >
            {m[0].toUpperCase() + m.slice(1)} · {i + 1}
          </button>
        ))}
      <button onClick={() => modelCommand('all')}>Select all · A</button>
      <button onClick={() => modelCommand('none')}>Select none · Alt+A</button>
      <button aria-pressed={s.boxSelect} onClick={() => modelCommand('b')}>
        Box select · B
      </button>
      {s.editMode && (
        <button aria-pressed={s.xray} onClick={() => s.set({ xray: !s.xray })}>
          X-ray · Alt+Z
        </button>
      )}
      {s.editMode && <button onClick={() => modelCommand('l')}>Select linked · L</button>}
      <span aria-label="Model selection">
        {s.editMode
          ? Object.values(s.elements).reduce(
              (n, v) =>
                n +
                (s.elementMode === 'face'
                  ? v.faces.length
                  : s.elementMode === 'edge'
                    ? v.edges.length
                    : v.vertices.length),
              0,
            ) + ' elements'
          : s.parts.length + ' parts'}
      </span>
      <span
        className={tris > 2048 || (s.liveVertices ?? 0) > 6144 ? 'issue' : ''}
        aria-label="Live DS budget"
      >
        {tris} / 2,048 triangles · {s.liveVertices ?? s.info?.budget.vertices ?? 0} / 6,144 vertices
      </span>
      {s.editMode && (
        <>
          <button disabled={busy || !writable} onClick={() => modelCommand('e')}>
            Extrude · E
          </button>
          <button disabled={busy || !writable} onClick={() => modelCommand('i')}>
            Inset · I
          </button>
          <button onClick={() => modelCommand('x')}>Delete · X</button>
          <button onClick={() => modelCommand('m')}>Merge · M</button>
        </>
      )}
      <span role="status" aria-label="Model operation">
        {s.modal ||
          'G move · R rotate · S scale · X/Y/Z axis · Ctrl snap · Enter confirm · Esc cancel'}
      </span>
      {s.editMenu && (
        <div
          className="model-menu"
          role="menu"
          aria-label={s.editMenu === 'delete' ? 'Delete mesh' : 'Merge mesh'}
        >
          {s.editMenu === 'delete' ? (
            ['Vertices', 'Edges', 'Faces'].map((v) => (
              <button
                role="menuitem"
                key={v}
                onClick={() => modelCommand('delete-' + v.toLowerCase())}
              >
                {v}
              </button>
            ))
          ) : (
            <button role="menuitem" onClick={() => modelCommand('merge-center')}>
              At center
            </button>
          )}
          <button role="menuitem" onClick={() => s.set({ editMenu: '' })}>
            Cancel
          </button>
        </div>
      )}
    </div>
  );
}

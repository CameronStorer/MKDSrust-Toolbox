import { useState } from 'react';
import { inspectScene, pickFile } from '../backend';
import { useStore } from '../store';
import { useScene } from '../scene-view';
import type { ModelAction, MeshSelection, SceneEntry } from '../bindings';
import { runModel, selectPart } from '../model-actions';

export function RigTools() {
  const { info, bone, rigBrush, set } = useScene();
  return (
    <div className="engine-tools">
      <div className="section-label">RIGID BONE ASSIGNMENT</div>
      <p>
        Drag over triangles to assign their vertices to one bone. Preview the base clips in the
        timeline.
      </p>
      <label>
        Assign to bone
        <select
          aria-label="Rig bone"
          value={bone}
          onChange={(e) => set({ bone: Number(e.target.value) })}
        >
          {info?.bones.map((b, i) => (
            <option key={i} value={i}>
              {i} · {b}
            </option>
          ))}
        </select>
      </label>
      <button aria-pressed={rigBrush} onClick={() => set({ rigBrush: !rigBrush })}>
        {rigBrush ? 'Stop painting weights' : 'Paint bone assignments'}
      </button>
    </div>
  );
}

export function ModelTools() {
  const { info, part, set, gizmo, triangle } = useScene();
  const busy = useStore((s) => s.busy);
  const project = useStore((s) => !!s.snapshot?.project_path);
  const [target, setTarget] = useState(512);
  const [height, setHeight] = useState(0);
  const [positive, setPositive] = useState(true);
  const [distance, setDistance] = useState(0.001);
  const [selection, setSelection] = useState('Part');
  const [bounds, setBounds] = useState('-1,-1,-1,1,1,1');
  const path = info?.model_path;
  const files = useStore((s) => s.snapshot?.files);
  const references =
    files?.filter(
      (f) => f.format === 'nsbmd' && (f.path.includes('/character/') || f.path.includes('/kart/')),
    ) ?? [];
  const placement = useScene();
  const current = info?.parts[part];
  const apply = (action: ModelAction) => {
    const parts = useScene.getState().parts;
    if (path)
      void runModel(
        action.kind === 'Import' ? [action] : parts.map((part) => ({ ...action, part })),
      );
  };
  async function importModel() {
    const source = await pickFile(['glb', 'gltf', 'obj']);
    if (source) apply({ kind: 'Import', source });
  }
  function remove() {
    let value: MeshSelection = { kind: 'Part' };
    if (selection === 'Connected') value = { kind: 'Connected', triangle };
    if (selection === 'Material') value = { kind: 'Material', material: current?.material ?? 0 };
    if (selection === 'Box') {
      const values = bounds.split(',').map(Number);
      if (values.length !== 6 || values.some((v) => !Number.isFinite(v))) {
        useStore.getState().set({ error: 'Enter six box coordinates: min x,y,z,max x,y,z' });
        return;
      }
      value = {
        kind: 'Box',
        minimum: [values[0], values[1], values[2]],
        maximum: [values[3], values[4], values[5]],
      };
    }
    apply({ kind: 'Delete', part, selection: value });
  }
  return (
    <div className="engine-tools">
      <div className="section-label">MODEL TOOLS</div>
      <fieldset disabled={!project || busy || !path}>
        <button onClick={() => void importModel()}>Import model…</button>
        <p className="muted">
          Replaces this model. GLB / glTF / OBJ; PNG and JPEG textures convert automatically.
        </p>
        <label>
          Part
          <select
            aria-label="Model part"
            value={part}
            onChange={(e) => selectPart(Number(e.target.value))}
          >
            {info?.parts.map((p, i) => (
              <option key={i} value={i}>
                {p.name} · {p.triangles} triangles
              </option>
            ))}
          </select>
        </label>
        <div className="engine-buttons">
          {(['off', 'translate', 'rotate', 'scale'] as const).map((mode) => (
            <button
              key={mode}
              className={gizmo === mode ? 'active' : ''}
              onClick={() => set({ gizmo: mode })}
            >
              {mode === 'off' ? 'Select' : mode[0].toUpperCase() + mode.slice(1)}
            </button>
          ))}
        </div>
        <p className="muted">
          Drag the gizmo to apply one undoable transform. Click a triangle to select a part.
        </p>
        <div className="engine-buttons">
          <button
            onClick={() => {
              if (path)
                void useStore.getState().run({
                  kind: 'Wizard',
                  action: { kind: 'AlignModel', path, ground: true, center: false },
                });
            }}
          >
            Snap to ground
          </button>
          <button
            onClick={() => {
              if (path)
                void useStore.getState().run({
                  kind: 'Wizard',
                  action: { kind: 'AlignModel', path, ground: false, center: true },
                });
            }}
          >
            Centre under driver
          </button>
        </div>
        <label>
          Seated reference
          <select
            aria-label="Seated reference"
            value={placement.reference}
            onChange={(e) => placement.set({ reference: e.target.value })}
          >
            <option value="">None</option>
            {references.map((f) => (
              <option key={f.path} value={f.path}>
                {f.path}
              </option>
            ))}
          </select>
        </label>
        <label>
          Kart slot
          <input
            aria-label="Placement kart slot"
            type="number"
            min={0}
            max={36}
            value={placement.referenceKart}
            onChange={(e) => placement.set({ referenceKart: Number(e.target.value) })}
          />
        </label>
        <label>
          Character slot
          <input
            aria-label="Placement character slot"
            type="number"
            min={0}
            max={12}
            value={placement.referenceCharacter}
            onChange={(e) => placement.set({ referenceCharacter: Number(e.target.value) })}
          />
        </label>
        <button
          aria-pressed={placement.pickWheels}
          onClick={() => placement.set({ pickWheels: !placement.pickWheels, wheelCenters: [] })}
        >
          {placement.pickWheels ? 'Stop picking wheel centres' : 'Pick four wheel centres'}
        </button>
        {placement.pickWheels && (
          <>
            <p>
              Click front left, front right, rear left, rear right. Rotate the view to reach each
              centre. Positions: {placement.wheelCenters.length} / 4.
            </p>
            <textarea
              aria-label="Picked wheel centres"
              readOnly
              value={JSON.stringify(placement.wheelCenters)}
            />
            <button
              disabled={placement.wheelCenters.length !== 4}
              onClick={() =>
                void useStore.getState().run({
                  kind: 'Wizard',
                  action: {
                    kind: 'SetWheels',
                    slot: placement.referenceKart,
                    wheels: placement.wheelCenters as [
                      [number, number, number],
                      [number, number, number],
                      [number, number, number],
                      [number, number, number],
                    ],
                  },
                })
              }
            >
              Apply wheel centres
            </button>
          </>
        )}
        <label>
          Target triangles: {Math.min(target, current?.triangles ?? target)} /{' '}
          {current?.triangles ?? 0}
          <input
            aria-label="Target triangles"
            type="range"
            min={1}
            max={Math.max(1, current?.triangles ?? 2048)}
            value={Math.min(target, current?.triangles ?? target)}
            onChange={(e) => setTarget(Number(e.target.value))}
          />
        </label>
        <button
          onClick={() =>
            apply({
              kind: 'Decimate',
              part,
              target: Math.min(target, current?.triangles ?? target),
            })
          }
        >
          Decimate
        </button>
        <label>
          Cut plane Y
          <input
            aria-label="Cut plane Y"
            type="number"
            step="0.1"
            value={height}
            onChange={(e) => setHeight(Number(e.target.value))}
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={positive}
            onChange={(e) => setPositive(e.target.checked)}
          />
          Keep above plane
        </label>
        <button
          onClick={() =>
            apply({ kind: 'Cut', part, normal: [0, 1, 0], distance: height, positive })
          }
        >
          Cut by plane
        </button>
        <label>
          Delete selection
          <select
            aria-label="Delete selection"
            value={selection}
            onChange={(e) => setSelection(e.target.value)}
          >
            {['Part', 'Material', 'Connected', 'Box'].map((v) => (
              <option key={v}>{v}</option>
            ))}
          </select>
        </label>
        {selection === 'Connected' && <p>Triangle {triangle} (click in viewport)</p>}
        {selection === 'Box' && (
          <label>
            Box min/max
            <input
              aria-label="Selection box"
              value={bounds}
              onChange={(e) => setBounds(e.target.value)}
            />
          </label>
        )}
        <button onClick={remove}>Delete selected</button>
        <label>
          Merge distance
          <input
            type="number"
            min={0}
            step="0.001"
            value={distance}
            onChange={(e) => setDistance(Number(e.target.value))}
          />
        </label>
        <div className="engine-buttons">
          <button onClick={() => apply({ kind: 'Merge', part, distance })}>Merge vertices</button>
          <button onClick={() => apply({ kind: 'Normals', part })}>Recompute normals</button>
        </div>
      </fieldset>
    </div>
  );
}

export function CollisionTools() {
  const { info, brush, attribute, set } = useScene();
  const busy = useStore((s) => s.busy);
  const project = useStore((s) => !!s.snapshot?.project_path);
  const files = useStore((s) => s.snapshot?.files);
  const [donor, setDonor] = useState('');
  const [objects, setObjects] = useState<SceneEntry[]>([]);
  const [object, setObject] = useState(0);
  const [position, setPosition] = useState<[number, number, number]>([0, 20, 0]);
  return (
    <div className="engine-tools">
      <div className="section-label">COLLISION TOOLS</div>
      <fieldset disabled={!project || busy || !info?.kcl_path}>
        <button
          disabled={!info?.model_path}
          onClick={() => {
            if (info?.model_path && info.kcl_path)
              void useStore
                .getState()
                .run({ kind: 'GenerateCollision', model: info.model_path, path: info.kcl_path });
          }}
        >
          Generate collision
        </button>
        <p className="muted">Uses material names and slope. Replaces the course KCL.</p>
        <label>
          Collision type
          <select
            aria-label="Collision type"
            value={(attribute >> 8) & 31}
            onChange={(e) => {
              const type = Number(e.target.value);
              set({
                attribute: (type << 8) | ([8, 9, 14, 16, 21].includes(type) ? 0x4000 : 0x8000),
              });
            }}
          >
            {[
              'Road',
              'Slippery road',
              'Weak offroad',
              'Offroad',
              'Sound trigger',
              'Heavy offroad',
              'Slippery road 2',
              'Boost pad',
              'Wall',
              'Invisible wall',
              'Out of bounds',
              'Fall boundary',
              'Jump pad',
              'Road without drivers',
              'Wall without drivers',
              'Cannon',
              'Edge wall',
              'Water fall',
              'Minimum speed boost',
              'Loop',
              'Special road',
              'Wall 3',
              'Recalculate route',
            ].map((name, i) => (
              <option key={name} value={i}>
                {i} · {name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Full attribute
          <input
            aria-label="Collision attribute"
            type="number"
            min={0}
            max={65535}
            value={attribute}
            onChange={(e) =>
              set({ attribute: Math.max(0, Math.min(65535, Number(e.target.value))) })
            }
          />
        </label>
        <button
          className={brush ? 'active' : ''}
          aria-pressed={brush}
          onClick={() => set({ brush: !brush })}
        >
          {brush ? 'Brush active' : 'Paint collision'}
        </button>
        <p className="muted">Drag across collision triangles. Each stroke is one undo step.</p>
        <div className="section-label">OBJECT PALETTE</div>
        <label>
          Object family
          <select
            aria-label="Object family"
            value={donor}
            onChange={(e) => {
              const path = e.target.value;
              setDonor(path);
              setObject(0);
              setObjects([]);
              if (path)
                void inspectScene(path)
                  .then((scene) =>
                    setObjects(
                      scene.entries.filter(
                        (e) => e.section === 'OBJI' && e.raw[38] === 255 && e.raw[39] === 255,
                      ),
                    ),
                  )
                  .catch((e: unknown) => useStore.getState().set({ error: String(e) }));
            }}
          >
            <option value="">Item box · global, safe anywhere</option>
            {files
              ?.filter((f) => f.path.startsWith('data/Course/') && f.name === 'course_map.nkm')
              .map((f) => (
                <option key={f.path} value={f.path}>
                  {f.path.split('/').at(-2)}
                </option>
              ))}
          </select>
        </label>
        {donor && (
          <label>
            Donor object
            <select
              aria-label="Donor object"
              value={object}
              onChange={(e) => setObject(Number(e.target.value))}
            >
              {objects.map((o) => (
                <option key={o.index} value={o.index}>
                  #{o.index} · type {o.raw[36] | (o.raw[37] << 8)}
                </option>
              ))}
            </select>
          </label>
        )}
        <div className="wizard-grid">
          {position.map((v, i) => (
            <label key={i}>
              {['X', 'Y', 'Z'][i]}
              <input
                aria-label={`Object ${['X', 'Y', 'Z'][i]}`}
                type="number"
                value={v}
                onChange={(e) =>
                  setPosition(
                    position.map((p, j) => (i === j ? Number(e.target.value) : p)) as [
                      number,
                      number,
                      number,
                    ],
                  )
                }
              />
            </label>
          ))}
        </div>
        <button
          disabled={!info?.nkm_path || (!!donor && !objects.length)}
          onClick={() => {
            if (info?.nkm_path)
              void useStore.getState().run({
                kind: 'Wizard',
                action: {
                  kind: 'PlaceObject',
                  path: info.nkm_path,
                  donor: donor || null,
                  index: donor
                    ? objects.some((o) => o.index === object)
                      ? object
                      : objects[0].index
                    : 0,
                  position,
                },
              });
          }}
        >
          Place object
        </button>
        <p>
          Donor objects bring their course model, texture and animation family. Routed objects
          require their routes first.
        </p>
      </fieldset>
    </div>
  );
}

import { useState } from 'react';
import type { Action } from './bindings';
import { pickFile } from './backend';
import { useStore } from './store';
import { openFile, openWorkspace } from './commands';
import { useScene } from './scene-view';

const flows = ['Character', 'Kart', 'Track', 'Music', 'Cups', 'Items', 'Names', 'Tables'] as const;
type WheelCentres = [
  [number, number, number],
  [number, number, number],
  [number, number, number],
  [number, number, number],
];
type Flow = (typeof flows)[number];
const characters = [
  'Mario',
  'Donkey Kong',
  'Toad',
  'Bowser',
  'Peach',
  'Wario',
  'Yoshi',
  'Luigi',
  'Dry Bones',
  'Daisy',
  'Waluigi',
  'R.O.B.',
  'Shy Guy',
];
const square: [number, number][] = [
  [0.2, 0.2],
  [0.8, 0.2],
  [0.8, 0.8],
  [0.2, 0.8],
];
export function Wizards() {
  const { snapshot, busy, run, set } = useStore();
  const [flow, setFlow] = useState<Flow>(useStore.getState().recipe);
  const [source, setSource] = useState('');
  const [slot, setSlot] = useState(0);
  const [base, setBase] = useState(0);
  const [hips, setHips] = useState(0.45);
  const [up, setUp] = useState('auto');
  const [tire, setTire] = useState('M');
  const [scale, setScale] = useState(1);
  const [speed, setSpeed] = useState(7.5);
  const [acceleration, setAcceleration] = useState(0.05);
  const [portraitImage, setPortraitImage] = useState('');
  const [portraitCell, setPortraitCell] = useState(0);
  const [wheels, setWheels] = useState('');
  const [seat, setSeat] = useState(0);
  const [course, setCourse] = useState(22);
  const [line, setLine] = useState<[number, number][]>(square);
  const [width, setWidth] = useState(150);
  const [laps, setLaps] = useState(3);
  const [rows, setRows] = useState(3);
  const [sequence, setSequence] = useState(16);
  const [bank, setBank] = useState(55);
  const [rate, setRate] = useState(16000);
  const [loopStart, setLoopStart] = useState('');
  const [loopEnd, setLoopEnd] = useState('');
  const [cup, setCup] = useState(0);
  const [courses, setCourses] = useState<number[]>(
    snapshot?.wizard_catalog.cups[0] ?? [1, 2, 3, 4],
  );
  const [query, setQuery] = useState('');
  const [nameIndex, setNameIndex] = useState(0);
  const [name, setName] = useState('');
  const [tiles, setTiles] = useState('data/Scene/Race_us.carc/race_s_o.NCGR');
  const [palette, setPalette] = useState('data/Scene/Race.carc/race_s_o.NCLR');
  const [cells, setCells] = useState('data/Scene/Race.carc/race_s.NCER');
  const [cell, setCell] = useState(0);
  const [address, setAddress] = useState('0x02154128');
  const [value, setValue] = useState('20');
  const [valueType, setValueType] = useState<'U8' | 'U16' | 'U32' | 'I32' | 'Fx32'>('U32');
  const [overlay, setOverlay] = useState('');
  const [uncompressed, setUncompressed] = useState(false);
  const catalog = snapshot?.wizard_catalog;
  const names = snapshot?.names ?? [];
  const filteredNames = names
    .map((n, index) => ({ ...n, indexInList: index }))
    .filter((n) => `${n.group} ${n.path} ${n.text}`.toLowerCase().includes(query.toLowerCase()));
  const courseOptions = catalog?.courses.filter((c) => c.id <= 40 && c.music_record !== null) ?? [];
  async function browse() {
    const extensions = ['Character', 'Kart', 'Track'].includes(flow)
      ? ['glb', 'gltf', 'obj']
      : flow === 'Music'
        ? ['wav', 'mid', 'midi']
        : ['png'];
    const file = await pickFile(extensions);
    if (file) setSource(file);
  }
  function number(
    label: string,
    value: number,
    change: (v: number) => void,
    min = 0,
    max = 10000,
    step = 1,
  ) {
    return (
      <label>
        {label}
        <input
          aria-label={label}
          type="number"
          value={value}
          min={min}
          max={max}
          step={step}
          onChange={(e) => change(Number(e.target.value))}
        />
      </label>
    );
  }
  function courseSelect(label: string, value: number, change: (v: number) => void) {
    return (
      <label>
        {label}
        <select aria-label={label} value={value} onChange={(e) => change(Number(e.target.value))}>
          {courseOptions.map((c) => (
            <option key={c.id} value={c.id}>
              {c.id} · {c.stem}
            </option>
          ))}
        </select>
      </label>
    );
  }
  async function apply() {
    let action: Action;
    if (flow === 'Character') {
      const h = Math.SQRT1_2;
      action = {
        kind: 'Character',
        options: {
          slot,
          base,
          source,
          hips,
          rotation:
            up === 'auto'
              ? [0, 0, 0, 0]
              : up === 'x'
                ? [0, 0, h, h]
                : up === 'z'
                  ? [-h, 0, 0, h]
                  : [0, 0, 0, 1],
        },
      };
    } else if (flow === 'Kart') {
      let points:
        | [
            [number, number, number],
            [number, number, number],
            [number, number, number],
            [number, number, number],
          ]
        | null = null;
      if (wheels.trim()) {
        const parsed: unknown = JSON.parse(wheels);
        if (
          !Array.isArray(parsed) ||
          parsed.length !== 4 ||
          !parsed.every(
            (p: unknown) =>
              Array.isArray(p) &&
              p.length === 3 &&
              p.every((v: unknown) => typeof v === 'number' && Number.isFinite(v)),
          )
        )
          throw new Error('Enter four wheel centres: [[x,y,z],…]');
        points = parsed as WheelCentres;
      }
      action = {
        kind: 'Kart',
        options: {
          slot,
          base,
          source,
          tire,
          wheels: points,
          seat: [0, seat, 0],
          translation: [0, 0, 0],
          rotation: [0, 0, 0, 1],
          scale,
        },
      };
    } else if (flow === 'Track')
      action = { kind: 'Track', options: { course, source, line, width, laps, item_rows: rows } };
    else if (flow === 'Music')
      action = {
        kind: 'Music',
        course,
        source,
        options: {
          sequence,
          bank,
          route: 'auto',
          rate,
          stereo: false,
          encoding: 2,
          loop_start: loopStart ? Number(loopStart) : null,
          loop_end: loopEnd ? Number(loopEnd) : null,
        },
      };
    else if (flow === 'Cups')
      action = { kind: 'Cup', slot: cup, courses: courses as [number, number, number, number] };
    else if (flow === 'Items')
      action = { kind: 'Icon', tiles, palette, cells, index: cell, source };
    else if (flow === 'Names') {
      const target = names[nameIndex];
      if (!target) return;
      action = { kind: 'Name', path: target.path, index: target.index, text: name };
    } else
      action = {
        kind: 'Table',
        overlay: overlay === '' ? null : Number(overlay),
        address: Number(address),
        values: [{ kind: valueType, value: Number(value) }],
        uncompressed,
      };
    await run({ kind: 'Wizard', action });
  }
  return (
    <div className="modal-backdrop">
      <section
        role="dialog"
        aria-modal="true"
        aria-label="Beginner recipes"
        className="wizard-dialog"
      >
        <header>
          <div>
            <span className="eyebrow">GUIDED AUTHORING</span>
            <h2>Make something for your DS</h2>
          </div>
          <button aria-label="Close recipes" onClick={() => set({ wizardOpen: false })}>
            ×
          </button>
        </header>
        <nav aria-label="Recipe types">
          {flows
            .filter((f) => f !== 'Tables' || useStore.getState().advanced)
            .map((f) => (
              <button
                key={f}
                className={flow === f ? 'active' : ''}
                onClick={() => {
                  setFlow(f);
                  setSource('');
                  setSlot(0);
                }}
              >
                {f}
              </button>
            ))}
        </nav>
        <div className="wizard-body engine-tools">
          {!snapshot?.manifest && (
            <p>
              Create a project to keep your work.{' '}
              <button onClick={() => set({ createOpen: true, wizardOpen: false })}>
                Create project?
              </button>
            </p>
          )}
          {['Character', 'Kart', 'Track', 'Music', 'Items'].includes(flow) && (
            <section>
              <h3>
                1 · Bring your own{' '}
                {flow === 'Music' ? 'audio' : flow === 'Items' ? 'image' : 'model'}
              </h3>
              <label>
                Source file
                <div className="wizard-file">
                  <input
                    aria-label="Recipe source"
                    value={source}
                    onChange={(e) => setSource(e.target.value)}
                  />
                  <button onClick={() => void browse()}>Browse…</button>
                </div>
              </label>
            </section>
          )}
          {(flow === 'Character' || flow === 'Kart') && (
            <section>
              <h3>2 · Choose a slot and defaults</h3>
              <div className="wizard-grid">
                {flow === 'Character' ? (
                  <>
                    <label>
                      Replace character
                      <select
                        aria-label="Replace character"
                        value={slot}
                        onChange={(e) => setSlot(Number(e.target.value))}
                      >
                        {characters.map((c, i) => (
                          <option key={c} value={i}>
                            {i} · {c}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      Base skeleton & clips
                      <select
                        aria-label="Base character"
                        value={base}
                        onChange={(e) => setBase(Number(e.target.value))}
                      >
                        {characters.map((c, i) => (
                          <option key={c} value={i}>
                            {i} · {c}
                          </option>
                        ))}
                      </select>
                    </label>
                  </>
                ) : (
                  <>
                    {number('Replace kart id', slot, setSlot, 0, 36)}
                    {number('Base kart id', base, setBase, 0, 36)}
                  </>
                )}
              </div>
            </section>
          )}
          {flow === 'Character' && (
            <section>
              <h3>3 · Fit, cut and rig</h3>
              <label>
                Source up axis
                <select
                  aria-label="Source up axis"
                  value={up}
                  onChange={(e) => setUp(e.target.value)}
                >
                  <option value="auto">Automatic (longest axis)</option>
                  <option value="y">Y up</option>
                  <option value="z">Z up</option>
                  <option value="x">X up</option>
                </select>
              </label>
              {number('Hip cut fraction', hips, setHips, 0, 0.94, 0.01)}
              <p>
                The remaining upper body fits the base. Automatic reduction keeps 237 triangles /
                536 vertices. Rigid weights follow the base skeleton; textures become 16 colours.
              </p>
              <p>
                After applying, use Model to adjust the mesh and Rig & Animate to preview the copied
                drive, spin, win and lose clips.
              </p>
            </section>
          )}
          {flow === 'Character' && (
            <section>
              <h3>4 · Portrait and icons</h3>
              <p>
                Render the authored model, or choose your own PNG. Preview the cell bank to confirm
                the destination and crop.
              </p>
              <label>
                Portrait image (optional)
                <input
                  aria-label="Portrait image"
                  value={portraitImage}
                  onChange={(e) => setPortraitImage(e.target.value)}
                />
              </label>
              <button
                onClick={() =>
                  void pickFile(['png']).then((p) => {
                    if (p) setPortraitImage(p);
                  })
                }
              >
                Choose portrait PNG…
              </button>
              {number('Portrait cell', portraitCell, setPortraitCell, 0, 1000)}
              <button
                onClick={() => {
                  set({ wizardOpen: false });
                  void openFile('Texture & 2D', 'data/Main2D_us.carc/player_character_L.nce.ncer');
                }}
              >
                Preview portrait cells…
              </button>
              <button
                disabled={
                  busy || !snapshot?.wizard_report?.files.some((p) => /\/P_[^/]+\.nsbmd$/.test(p))
                }
                onClick={() => {
                  const model = snapshot?.wizard_report?.files.find((p) =>
                    /\/P_[^/]+\.nsbmd$/.test(p),
                  );
                  if (!model) return;
                  const target = {
                    tiles: 'data/Main2D_us.carc/player_character_L.nce.ncgr',
                    palette: 'data/Main2D_us.carc/player_character_L_o.NCLR',
                    cells: 'data/Main2D_us.carc/player_character_L.nce.ncer',
                    index: portraitCell,
                  };
                  void run({
                    kind: 'Wizard',
                    action: portraitImage
                      ? { kind: 'Icon', ...target, source: portraitImage }
                      : { kind: 'Portrait', ...target, model },
                  });
                }}
              >
                Apply portrait to cell
              </button>
            </section>
          )}
          {flow === 'Kart' && (
            <section>
              <h3>3 · Placement and wheels</h3>
              <div className="wizard-grid">
                {number('Body scale', scale, setScale, 0.01, 10, 0.01)}
                {number('Seat height adjustment', seat, setSeat, -10, 10, 0.01)}
                <label>
                  Tire size
                  <select
                    aria-label="Tire size"
                    value={tire}
                    onChange={(e) => setTire(e.target.value)}
                  >
                    {['S', 'M', 'L'].map((t) => (
                      <option key={t}>{t}</option>
                    ))}
                  </select>
                </label>
              </div>
              <label>
                Four wheel centres (optional)
                <textarea
                  aria-label="Wheel centres"
                  placeholder="[[x,y,z], [x,y,z], [x,y,z], [x,y,z]]"
                  value={wheels}
                  onChange={(e) => setWheels(e.target.value)}
                />
              </label>
              <p>
                Order: front left, front right, rear left, rear right. Leave empty to keep the base
                wheels. Body, stats and all 13 seat offsets are kept together.
              </p>
            </section>
          )}
          {flow === 'Track' && (
            <>
              <section>
                <h3>2 · Select a course</h3>
                {courseSelect('Replace course', course, setCourse)}
                <p>
                  Fits the model inside the slot’s minimap rectangle. Sky stays from the base.
                  Collision follows materials and slope.
                </p>
              </section>
              <section>
                <h3>3 · Draw the racing line</h3>
                <p>
                  Click to add points. The last point connects to the first. Use the same clockwise
                  or counterclockwise direction throughout.
                </p>
                <svg
                  className="line-editor"
                  role="img"
                  aria-label="Draw racing line"
                  viewBox="0 0 400 250"
                  preserveAspectRatio="none"
                  onClick={(e) => {
                    const r = e.currentTarget.getBoundingClientRect();
                    setLine([
                      ...line,
                      [(e.clientX - r.left) / r.width, (e.clientY - r.top) / r.height],
                    ]);
                  }}
                >
                  <defs>
                    <pattern id="recipe-grid" width="25" height="25" patternUnits="userSpaceOnUse">
                      <path d="M25 0H0V25" fill="none" stroke="currentColor" opacity="0.15" />
                    </pattern>
                  </defs>
                  <rect width="400" height="250" fill="url(#recipe-grid)" />
                  <polygon
                    points={line.map(([x, y]) => `${x * 400},${y * 250}`).join(' ')}
                    fill="none"
                    stroke="var(--accent)"
                    strokeWidth="2"
                  />
                  {line.map(([x, y], i) => (
                    <g key={i}>
                      <circle cx={x * 400} cy={y * 250} r="5" fill="var(--accent)" />
                      <text x={x * 400 + 8} y={y * 250 - 8} fill="currentColor">
                        {i + 1}
                      </text>
                    </g>
                  ))}
                </svg>
                <div className="button-row">
                  <button onClick={() => setLine([])}>Clear line</button>
                  <button onClick={() => setLine(line.slice(0, -1))}>Remove last point</button>
                  <button onClick={() => setLine(square)}>Default loop</button>
                </div>
                <div className="wizard-grid">
                  {number('Route width', width, setWidth, 10, 2000)}
                  {number('Laps', laps, setLaps, 1, 9)}
                  {number('Item box rows', rows, setRows, 0, 32)}
                </div>
                <p>
                  Creates 64 checkpoints, CPU and item routes, respawns, start positions and a
                  camera. All generated points remain editable in Track.
                </p>
              </section>
            </>
          )}
          {flow === 'Kart' && (
            <section>
              <h3>4 · Tune driving</h3>
              <p>
                Copy base stats with Apply kart, then adjust the destination. Values apply to race
                and menu tables.
              </p>
              {number('Top speed', speed, setSpeed, 0.1, 30, 0.1)}
              {number('Acceleration', acceleration, setAcceleration, 0.0001, 2, 0.001)}
              <button
                onClick={() => {
                  const row = catalog?.karts.find((k) => k.id === slot);
                  if (row) {
                    setSpeed(row.speed);
                    setAcceleration(row.acceleration);
                  }
                }}
              >
                Read current stats
              </button>
              <button
                disabled={busy || !snapshot?.manifest}
                onClick={() =>
                  void run({
                    kind: 'Wizard',
                    action: { kind: 'KartStats', slot, speed, acceleration },
                  })
                }
              >
                Apply driving stats
              </button>
            </section>
          )}
          {flow === 'Music' && (
            <section>
              <h3>2 · Course music and memory</h3>
              {courseSelect('Music course', course, setCourse)}
              <div className="wizard-grid">
                {number('Sequence to replace', sequence, setSequence, 0, 255)}
                {number('MIDI instrument bank', bank, setBank, 0, 255)}
                {number('Sample rate', rate, setRate, 8000, 32768, 1000)}
                <label>
                  Loop start sample
                  <input
                    aria-label="Recipe loop start"
                    value={loopStart}
                    onChange={(e) => setLoopStart(e.target.value)}
                  />
                </label>
                <label>
                  Loop end sample
                  <input
                    aria-label="Recipe loop end"
                    value={loopEnd}
                    onChange={(e) => setLoopEnd(e.target.value)}
                  />
                </label>
              </div>
              <p>Memory use is reported after import.</p>
              <p>
                700 KiB sound heap · aim under 250 KiB of samples. WAV uses mono ADPCM; MIDI uses
                your chosen instrument bank. Songs sharing a sequence slot will also change.
              </p>
              <button
                onClick={() => {
                  set({ wizardOpen: false });
                  openWorkspace('Audio');
                }}
              >
                Browse & replace sound effects…
              </button>
            </section>
          )}
          {flow === 'Cups' && (
            <section>
              <h3>Choose four races</h3>
              {number(
                'Cup slot',
                cup,
                (v) => {
                  setCup(v);
                  setCourses(catalog?.cups[v] ?? courses);
                },
                0,
                7,
              )}
              <div className="wizard-grid">
                {courses.map((c, i) => (
                  <div key={i}>
                    {courseSelect(`Race ${i + 1}`, c, (v) =>
                      setCourses(courses.map((old, j) => (j === i ? v : old))),
                    )}
                  </div>
                ))}
              </div>
              <p>
                Course-name messages follow the chosen courses in every available language. Rename
                the cup in Names. Open its name sprites in Texture & 2D for the selection headings.
              </p>
              <button
                onClick={() => {
                  set({ wizardOpen: false });
                  void openFile(
                    'Texture & 2D',
                    'data/Scene/Menu_us.carc/common/select_cup_name_m.nce.ncgr',
                  );
                }}
              >
                Edit cup name sprites…
              </button>
            </section>
          )}
          {flow === 'Items' && (
            <section>
              <h3>2 · Replace one HUD cell</h3>
              <label>
                Tile atlas
                <input
                  aria-label="Icon tiles"
                  value={tiles}
                  onChange={(e) => setTiles(e.target.value)}
                />
              </label>
              <label>
                Palette
                <input
                  aria-label="Icon palette"
                  value={palette}
                  onChange={(e) => setPalette(e.target.value)}
                />
              </label>
              <label>
                Cell bank
                <input
                  aria-label="Icon cells"
                  value={cells}
                  onChange={(e) => setCells(e.target.value)}
                />
              </label>
              {number('Cell index', cell, setCell, 0, 1000)}
              <p>
                The image fits the cell using its existing palette. Preview the cell bank in Texture
                & 2D to identify the icon before replacing it.
              </p>
              <button
                onClick={() => {
                  set({ wizardOpen: false });
                  void openFile('Texture & 2D', cells);
                }}
              >
                Preview HUD cells…
              </button>
              <h3>Item model textures</h3>
              <select
                aria-label="Item model"
                defaultValue=""
                onChange={(e) => {
                  set({ wizardOpen: false });
                  void openFile('Texture & 2D', e.target.value);
                }}
              >
                <option value="" disabled>
                  Choose an item to recolour or import a texture…
                </option>
                {snapshot?.files
                  .filter((f) => f.path.includes('/Item/') && f.format === 'nsbmd')
                  .map((f) => (
                    <option key={f.path} value={f.path}>
                      {f.name}
                    </option>
                  ))}
              </select>
            </section>
          )}
          {flow === 'Names' && (
            <section>
              <h3>Course, cup, character and kart names</h3>
              <input
                aria-label="Search names"
                placeholder="Search names or language…"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
              <div className="name-list">
                {filteredNames.map((n) => (
                  <button
                    className={nameIndex === n.indexInList ? 'active' : ''}
                    key={`${n.path}:${n.index}`}
                    onClick={() => {
                      setNameIndex(n.indexInList);
                      setName(n.text);
                    }}
                  >
                    <small>
                      {n.group} · {n.path.split('/').at(-2)} · #{n.index}
                    </small>
                    {n.text}
                  </button>
                ))}
              </div>
              <label>
                New name
                <input
                  aria-label="New name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
            </section>
          )}
          {flow === 'Tables' && (
            <section>
              <h3>ARM9 & overlay table editor</h3>
              <p>
                Addresses refer to the loaded binary. Cup table: 0x02154128 · music records:
                0x0215316C. Use Cups or Music for guided edits.
              </p>
              <label>
                Overlay id (empty = ARM9)
                <input
                  aria-label="Overlay id"
                  value={overlay}
                  onChange={(e) => setOverlay(e.target.value)}
                />
              </label>
              <div className="wizard-grid">
                <label>
                  RAM address
                  <input
                    aria-label="RAM address"
                    value={address}
                    onChange={(e) => setAddress(e.target.value)}
                  />
                </label>
                <label>
                  Type
                  <select
                    aria-label="Table value type"
                    value={valueType}
                    onChange={(e) => setValueType(e.target.value as typeof valueType)}
                  >
                    {['U8', 'U16', 'U32', 'I32', 'Fx32'].map((t) => (
                      <option key={t}>{t}</option>
                    ))}
                  </select>
                </label>
                <label>
                  Value
                  <input
                    aria-label="Table value"
                    value={value}
                    onChange={(e) => setValue(e.target.value)}
                  />
                </label>
              </div>
              <label>
                <input
                  type="checkbox"
                  checked={uncompressed}
                  onChange={(e) => setUncompressed(e.target.checked)}
                />{' '}
                Store selected binary uncompressed
              </label>
              <p>
                Export rebuilds the executable and module parameters. BPS carries executable edits.
              </p>
            </section>
          )}
          {(flow === 'Character' || flow === 'Kart') && (
            <section>
              <button
                disabled={busy}
                onClick={() => {
                  const model = snapshot?.edited.find((p) =>
                    flow === 'Character'
                      ? /\/P_[^/]+\.nsbmd$/.test(p)
                      : /\/kart_[^/]+\.nsbmd$/.test(p),
                  );
                  if (!model) {
                    set({ error: 'Apply the recipe first, then open its placement preview.' });
                    return;
                  }
                  const reference =
                    snapshot?.files.find(
                      (f) =>
                        f.format === 'nsbmd' &&
                        (flow === 'Character'
                          ? f.path.includes('/kart/mario/kart_MR_a.nsbmd')
                          : f.path.includes('/character/mario/P_MR.nsbmd')),
                    )?.path ?? '';
                  useScene.getState().set({
                    reference,
                    referenceKart: flow === 'Kart' ? slot : 0,
                    referenceCharacter: flow === 'Character' ? slot : 0,
                  });
                  set({ wizardOpen: false });
                  void openFile(flow === 'Character' ? 'Rig & Animate' : 'Model', model);
                }}
              >
                {flow === 'Character' ? 'Preview seated & animations' : 'Open placement view'}
              </button>
            </section>
          )}
          {snapshot?.wizard_report && (
            <section className="recipe-result" aria-live="polite">
              <h3>Applied · {snapshot.wizard_report.files.length} files</h3>
              {snapshot.wizard_report.warnings.map((w) => (
                <p key={w}>{w}</p>
              ))}
              <details>
                <summary>Changed files</summary>
                {snapshot.wizard_report.files.map((p) => (
                  <div key={p}>{p}</div>
                ))}
              </details>
            </section>
          )}
        </div>
        <footer>
          <span>One undoable change · check the result in your emulator</span>
          <button onClick={() => set({ wizardOpen: false, exportOpen: true })}>
            Export for DS…
          </button>
          <button
            className="primary"
            disabled={
              busy ||
              !snapshot?.manifest ||
              (['Character', 'Kart', 'Track', 'Music', 'Items'].includes(flow) && !source) ||
              (flow === 'Track' && line.length < 3)
            }
            onClick={() => void apply().catch((e: unknown) => set({ error: String(e) }))}
          >
            {busy ? 'Building…' : `Apply ${flow.toLowerCase()}`}
          </button>
        </footer>
      </section>
    </div>
  );
}

// The source JSON and image are emitted by Rust's synthetic fixture example.
import demo from './demo.json';
import assets from './demo-assets.json';
import sceneData from './demo-scene.json';
import audioData from './demo-audio.json';
let sceneState = structuredClone(sceneData);
import type { SceneInfo } from './bindings';
export async function scene(path: string): Promise<SceneInfo> {
  const tuple = (v: number[] | null): [number, number, number] | null =>
    v ? [v[0], v[1], v[2]] : null;
  return {
    ...clone(sceneState),
    path,
    entries: sceneState.entries.map((e) => ({
      ...e,
      position: tuple(e.position),
      end: tuple(e.end),
      fields: e.fields.map(([a, b]) => [a, b]),
    })),
  };
}
import { rgb555 } from './colors';
import type { Request, Snapshot, BinaryRequest, TextToken } from './bindings';
const clone = <T>(v: T): T => structuredClone(v);
const state = clone(demo) as Snapshot;
state.bench.games = [
  { id: 'studio', name: 'Synthetic native game', path: '/MKDSrust/games/studio' },
];
state.settings.play_target = 'mkdsrust';
let pixels = [...assets.pixels];
let colors = [...state.preview.colors];
const original = [...colors];
const messages = clone(assets.messages) as Record<string, typeof state.preview.messages>;
const originalMessages = clone(messages);
interface Edit {
  oldScene: typeof sceneState;
  newScene: typeof sceneState;
  oldAudio: typeof state.audio_resources;
  newAudio: typeof state.audio_resources;
  title: string;
  before: typeof colors;
  after: typeof colors;
  oldMessages: typeof messages;
  newMessages: typeof messages;
  oldEdited: string[];
  newEdited: string[];
  oldManifest: Snapshot['manifest'];
  newManifest: Snapshot['manifest'];
  oldPixels: number[];
  newPixels: number[];
}
let history: Edit[] = [];
let cursor = 0;
if (new URLSearchParams(location.search).get('onboarding') === '1') {
  Object.assign(state, {
    rom_path: '',
    project_path: '',
    manifest: null,
    files: [],
    edited: [],
    selected: '',
  });
}
function updatePreview() {
  if (state.selected.endsWith('.sdat') && !state.audio_resources.length)
    state.audio_resources = clone(audioData);
  const file = state.files.find((f) => f.path === state.selected);
  state.preview = {
    cells: file?.format === 'ncer' ? [1, 1] : [],
    sequences:
      file?.format === 'nanr'
        ? [
            [
              { cell: 0, duration: 12 },
              { cell: 1, duration: 12 },
            ],
          ]
        : [],
    palette_stride: 256,
    colors: file?.format === 'nclr' ? colors : [],
    messages: clone(messages[state.selected] ?? []),
    textures: file?.format === 'nsbtx' ? ['studio'] : [],
    palettes: file?.format === 'nsbtx' ? ['studio_pl'] : [],
    width: file?.format === 'nsbtx' ? 8 : assets.width,
    height: file?.format === 'nsbtx' ? 8 : assets.height,
  };
  state.history = history.map((h, i) => ({ title: h.title, applied: i < cursor }));
  state.can_undo = cursor > 0;
  state.can_redo = cursor < history.length;
}
export async function dispatch(r: Request): Promise<Snapshot> {
  const oldPixels = clone(pixels);
  const oldScene = clone(sceneState),
    oldAudio = clone(state.audio_resources);
  const before = clone(colors),
    oldMessages = clone(messages),
    oldEdited = clone(state.edited),
    oldManifest = clone(state.manifest);
  let editedPath = '';
  let title = '';
  switch (r.kind) {
    case 'Model':
      title = r.action.kind === 'Import' ? 'Import model' : r.action.kind + ' model';
      editedPath = r.path;
      if (r.action.kind === 'Decimate') {
        const part = sceneState.parts[r.action.part];
        sceneState.budget.polygons -= Math.max(0, part.triangles - r.action.target);
        part.triangles = Math.min(part.triangles, r.action.target);
      }
      break;
    case 'GenerateCollision':
    case 'PaintCollision':
      title = r.kind === 'GenerateCollision' ? 'Generate collision' : 'Paint collision';
      editedPath = r.path;
      break;
    case 'ImportSong':
      title = 'Import song';
      editedPath = r.path;
      state.song_report = {
        ram_bytes: 8192,
        loop_sample: r.options.loop_start,
        route: r.options.route === 'midi' ? 'midi' : 'sampled',
      };
      state.audio_resources.push({
        kind: 2,
        index: state.audio_resources.filter((r) => r.kind === 2).length,
        name: 'Imported bank',
        file: 3,
        bytes: 100,
        samples: 0,
      });
      break;
    case 'ReplaceEffect':
      title = 'Replace sound effect';
      editedPath = r.path;
      break;
    case 'Courses':
      state.bench.courses = [
        { id: 0, name: 'studio_course' },
        { id: 1, name: 'synthetic_circuit' },
      ];
      break;
    case 'Paint':
      for (const [x, y, c] of r.points) pixels[y * assets.width + x] = c;
      editedPath = r.path;
      title = 'Paint pixels';
      break;
    case 'ImportGraphic':
      state.graphic_import = {
        width: 128,
        height: 128,
        colors: 256,
        texture_bytes: 16384,
        palette_bytes: 512,
      };
      break;
    case 'ApplyGraphic':
      pixels = pixels.map((p) => p ^ 1);
      state.graphic_import = null;
      editedPath = state.selected;
      title = 'Import PNG';
      break;
    case 'CancelGraphic':
      state.graphic_import = null;
      break;
    case 'ShowInFolder':
      break;
    case 'ExportDs':
      state.ds_export = {
        changed: state.edited,
        in_place: state.edited,
        moved: [],
        rom_size: 1048576,
        rom_written: r.rom,
        patch_written: r.bps,
        patch_size: r.bps ? 1024 : null,
        warnings: [],
      };
      break;
    case 'CreateFromBase':
      state.edited = [];
      state.selected = '';
      state.pack = null;
      history = [];
      cursor = 0;
      state.base = {
        clean_sha256: state.rom_hash,
        patch: 'base.bps',
        source_name: r.source.split('/').pop() ?? 'synthetic.bps',
        source_sha256: '1234'.repeat(16),
      };
      state.base_changes = [{ path: 'data/Studio/readout.bmg', change: 'changed' }];
      state.project_path = r.path;
      state.manifest = r.manifest;
      break;
    case 'Reload':
    case 'Play':
      state.bench.running = true;
      state.bench.logs = [
        '[stdout] Synthetic native game started.\n',
        '[stdout] Mounted studio.colour-study. Ready.\n',
      ];
      state.bench.last_run = {
        package_size: 1248,
        install_path: '/MKDSrust/games/studio/mods/studio.colour-study',
        executable: '/MKDSrust/games/studio/game',
        exit_code: null,
        stopped: false,
      };
      break;
    case 'Stop':
      state.bench.running = false;
      state.bench.logs.push('[stdout] Stopped by Toolbox.\n');
      if (state.bench.last_run) {
        state.bench.last_run.exit_code = 0;
        state.bench.last_run.stopped = true;
      }
      break;
    case 'Snapshot':
      break;
    case 'Select':
      if (!state.files.some((f) => f.path === r.path)) throw new Error('File not found');
      state.selected = r.path;
      break;
    case 'Palette':
      r.indices.forEach((index, i) => {
        colors[index] = r.colors[i];
      });
      editedPath = r.path;
      title = 'Edit palette colours';
      break;
    case 'Text': {
      const m = messages[r.path][r.index];
      m.tokens = clone(r.tokens);
      m.preview = r.tokens
        .map((t: TextToken) => (t.kind === 'Text' ? t.value : '〈code〉'))
        .join('');
      editedPath = r.path;
      title = 'Edit text message';
      break;
    }
    case 'Revert':
      if (r.path.endsWith('nclr')) colors = [...original];
      if (originalMessages[r.path]) messages[r.path] = clone(originalMessages[r.path]);
      state.edited = state.edited.filter((p) => p !== r.path);
      title = 'Revert file';
      break;
    case 'UpdateManifest':
      state.manifest = clone(r.manifest);
      title = 'Edit project information';
      break;
    case 'Undo':
      if (cursor > 0) {
        const h = history[--cursor];
        pixels = clone(h.oldPixels);
        sceneState = clone(h.oldScene);
        state.audio_resources = clone(h.oldAudio);
        colors = clone(h.before);
        Object.assign(messages, clone(h.oldMessages));
        state.edited = clone(h.oldEdited);
        state.manifest = clone(h.oldManifest);
      }
      break;
    case 'Redo':
      if (cursor < history.length) {
        const h = history[cursor++];
        pixels = clone(h.newPixels);
        sceneState = clone(h.newScene);
        state.audio_resources = clone(h.newAudio);
        colors = clone(h.after);
        Object.assign(messages, clone(h.newMessages));
        state.edited = clone(h.newEdited);
        state.manifest = clone(h.newManifest);
      }
      break;
    case 'Settings':
      state.settings = clone(r.settings);
      break;
    case 'Validate':
      state.issues = [];
      break;
    case 'Pack':
      if (!state.edited.length) throw new Error('There are no changed files to pack');
      state.pack = {
        path: `${state.project_path}/build/${state.manifest?.id}-${state.manifest?.version}.zip`,
        size: 742,
        warnings: ['Synthetic preview: no package was written.'],
      };
      break;
    case 'CreateProject':
      state.base = null;
      state.base_changes = [];
      state.ds_export = null;
      state.manifest = clone(r.manifest);
      state.project_path = r.path;
      state.edited = [];
      history = [];
      cursor = 0;
      break;
    case 'OpenProject':
      state.project_path = r.path;
      state.manifest = clone(demo.manifest);
      break;
    case 'OpenRom':
      state.rom_path = r.path;
      state.files = clone(demo.files);
      state.project_path = '';
      state.manifest = null;
      state.edited = [];
      state.selected = '';
      history = [];
      cursor = 0;
      break;
    case 'Export':
    case 'Extract':
      break;
  }
  if (editedPath && !state.edited.includes(editedPath)) state.edited.push(editedPath);
  if (title) {
    history = history.slice(0, cursor);
    history.push({
      title,
      oldScene,
      newScene: clone(sceneState),
      oldAudio,
      newAudio: clone(state.audio_resources),
      before,
      after: clone(colors),
      oldMessages,
      newMessages: clone(messages),
      oldEdited,
      newEdited: clone(state.edited),
      oldManifest,
      newManifest: clone(state.manifest),
      oldPixels,
      newPixels: clone(pixels),
    });
    cursor++;
  }
  updatePreview();
  return clone(state);
}
export async function binary(r: BinaryRequest): Promise<ArrayBuffer> {
  if (r.mode === 'audio-wav') return (await fetch('/demo/tone.wav')).arrayBuffer();
  if (r.mode === 'glb' || r.mode === 'course-glb')
    return (await fetch(`/demo/${r.mode === 'glb' ? 'model' : 'course'}.glb`)).arrayBuffer();
  if (r.mode === 'palette-colors') return Uint16Array.from(colors).buffer;
  if (r.mode === 'paint-data') {
    const b = new ArrayBuffer(12 + pixels.length * 2);
    const d = new DataView(b);
    d.setUint32(0, assets.width, true);
    d.setUint32(4, assets.height, true);
    d.setUint32(8, 0, true);
    new Uint16Array(b, 12).set(pixels);
    return b;
  }
  if (r.mode === 'cell-png') return (await fetch('/demo/cell.png')).arrayBuffer();
  if (r.mode === 'png' || r.mode === 'import-preview') {
    if (r.path.endsWith('.nsbtx')) return (await fetch('/demo/texture.png')).arrayBuffer();
    const paletteOnly = r.path.endsWith('.nclr');
    const canvas = document.createElement('canvas');
    canvas.width = paletteOnly ? 16 : assets.width;
    canvas.height = paletteOnly ? Math.ceil(colors.length / 16) : assets.height;
    const context = canvas.getContext('2d')!;
    const image = context.createImageData(canvas.width, canvas.height);
    for (let i = 0; i < canvas.width * canvas.height; i++) {
      const index = paletteOnly ? i : pixels[i];
      const color = rgb555(colors[index] ?? 0).map((c) => (c << 3) | (c >> 2));
      image.data.set([...color, !paletteOnly && index === 0 ? 0 : 255], i * 4);
    }
    context.putImageData(image, 0, 0);
    const blob = await new Promise<Blob>((resolve, reject) =>
      canvas.toBlob(
        (b) => (b ? resolve(b) : reject(new Error('PNG encoding failed'))),
        'image/png',
      ),
    );
    return blob.arrayBuffer();
  }
  const files: Record<string, number[]> = assets.hex;
  return Uint8Array.from((files[r.path] ?? []).slice(r.start, r.start + Math.min(r.length, 65536)))
    .buffer;
}
export async function inspect(path: string): Promise<Snapshot['preview']> {
  const previous = state.selected;
  state.selected = path;
  updatePreview();
  const result = clone(state.preview);
  state.selected = previous;
  updatePreview();
  return result;
}

// The source JSON and image are emitted by Rust's synthetic fixture example.
import demo from './demo.json';
import assets from './demo-assets.json';
import sceneData from './demo-scene.json';
import type { SceneInfo } from './bindings';
export async function scene(path: string): Promise<SceneInfo> { return { ...clone(sceneData) as SceneInfo, path }; }
import { rgb555 } from './colors';
import type { Request, Snapshot, BinaryRequest, TextToken } from './bindings';
const clone = <T>(v: T): T => structuredClone(v);
const state = clone(demo) as Snapshot;
state.bench.games = [{ id: 'studio', name: 'Synthetic native game', path: '/MKDSrust/games/studio' }];
let colors = [...state.preview.colors];
const original = [...colors];
const messages = clone(assets.messages) as Record<string, typeof state.preview.messages>;
const originalMessages = clone(messages);
interface Edit {
  title: string;
  before: typeof colors;
  after: typeof colors;
  oldMessages: typeof messages;
  newMessages: typeof messages;
  oldEdited: string[];
  newEdited: string[];
  oldManifest: Snapshot['manifest'];
  newManifest: Snapshot['manifest'];
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
  const file = state.files.find((f) => f.path === state.selected);
  state.preview = {
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
  const before = clone(colors),
    oldMessages = clone(messages),
    oldEdited = clone(state.edited),
    oldManifest = clone(state.manifest);
  let editedPath = '';
  let title = '';
  switch (r.kind) {
    case 'Play':
      state.bench.running = true;
      state.bench.logs = ['[stdout] Synthetic native game started.\n', '[stdout] Mounted studio.colour-study. Ready.\n'];
      state.bench.last_run = { package_size: 1248, install_path: '/MKDSrust/games/studio/mods/studio.colour-study', executable: '/MKDSrust/games/studio/game', exit_code: null, stopped: false };
      break;
    case 'Stop':
      state.bench.running = false;
      state.bench.logs.push('[stdout] Stopped by Toolbox.\n');
      if (state.bench.last_run) { state.bench.last_run.exit_code = 0; state.bench.last_run.stopped = true; }
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
        colors = clone(h.before);
        Object.assign(messages, clone(h.oldMessages));
        state.edited = clone(h.oldEdited);
        state.manifest = clone(h.oldManifest);
      }
      break;
    case 'Redo':
      if (cursor < history.length) {
        const h = history[cursor++];
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
      before,
      after: clone(colors),
      oldMessages,
      newMessages: clone(messages),
      oldEdited,
      newEdited: clone(state.edited),
      oldManifest,
      newManifest: clone(state.manifest),
    });
    cursor++;
  }
  updatePreview();
  return clone(state);
}
export async function binary(r: BinaryRequest): Promise<ArrayBuffer> {
  if (r.mode === 'glb' || r.mode === 'course-glb') return (await fetch(`/demo/${r.mode === 'glb' ? 'model' : 'course'}.glb`)).arrayBuffer();
  if (r.mode === 'png') {
    if (r.path.endsWith('.nsbtx')) return (await fetch('/demo/texture.png')).arrayBuffer();
    const paletteOnly = r.path.endsWith('.nclr');
    const canvas = document.createElement('canvas');
    canvas.width = paletteOnly ? 16 : assets.width;
    canvas.height = paletteOnly ? Math.ceil(colors.length / 16) : assets.height;
    const context = canvas.getContext('2d')!;
    const image = context.createImageData(canvas.width, canvas.height);
    for (let i = 0; i < canvas.width * canvas.height; i++) {
      const index = paletteOnly ? i : assets.pixels[i];
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

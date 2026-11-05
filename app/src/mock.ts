// The source JSON and image are emitted by Rust's synthetic fixture example.
import demo from './demo.json';
import assets from './demo-assets.json';
import sceneData from './demo-scene.json';
import audioData from './demo-audio.json';
import wizardData from './demo-wizards.json';
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { GLTFExporter } from 'three/examples/jsm/exporters/GLTFExporter.js';
import { previewEdit } from './mesh-selection';
let sceneState = structuredClone(sceneData) as unknown as SceneInfo;
import type { SceneInfo } from './bindings';
let sceneBank = structuredClone(wizardData.scenes) as unknown as Record<string, SceneInfo>;
let modelBytes: Record<string, ArrayBuffer> = {};
export async function scene(path: string): Promise<SceneInfo> {
  const fixture = sceneBank[path];
  if (fixture) return clone(fixture);
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
state.pairings.push(...clone(wizardData.pairings));
state.related_pages = clone(wizardData.related_pages) as Snapshot['related_pages'];
state.wizard_catalog = clone(wizardData.catalog);
state.names = clone(wizardData.names);
state.game_entries.push(
  ...clone(wizardData.game_entries).filter(
    (e) => !state.game_entries.some((old) => old.path === e.path),
  ),
);
state.files.push(
  ...clone(wizardData.files).filter((f) => !state.files.some((old) => old.path === f.path)),
);
state.bench.games = [
  { id: 'studio', name: 'Synthetic native game', path: '/MKDSrust/games/studio' },
];
state.settings.play_target = 'mkdsrust';
let pixels = [...assets.pixels];
let colors = [...state.preview.colors];
const original = [...colors];
const messages = { ...clone(assets.messages), ...clone(wizardData.messages) } as Record<
  string,
  typeof state.preview.messages
>;
const originalMessages = clone(messages);
interface Edit {
  oldSceneBank: typeof sceneBank;
  newSceneBank: typeof sceneBank;
  oldModelBytes: typeof modelBytes;
  newModelBytes: typeof modelBytes;
  paths: string[];
  oldCatalog: Snapshot['wizard_catalog'];
  newCatalog: Snapshot['wizard_catalog'];
  oldNames: Snapshot['names'];
  newNames: Snapshot['names'];
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
  state.history = history.map((h, i) => ({
    title: h.title,
    applied: i < cursor,
    paths: h.paths,
  }));
  state.can_undo = cursor > 0;
  state.can_redo = cursor < history.length;
}
export async function dispatch(r: Request): Promise<Snapshot> {
  const oldSceneBank = clone(sceneBank),
    oldModelBytes = clone(modelBytes);
  let changePaths: string[] = [];
  const oldCatalog = clone(state.wizard_catalog),
    oldNames = clone(state.names);
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
    case 'Wizard': {
      const a = r.action;
      title = 'Beginner recipe';
      let files: string[] = [];
      if (a.kind === 'Character')
        files = state.files
          .filter((f) => f.path.includes('/character/') && /\.nsb(md|tx|ca)$/.test(f.path))
          .slice(0, 10)
          .map((f) => f.path);
      else if (a.kind === 'Kart')
        files = state.files
          .filter((f) => f.path.includes('/kart/') && f.path.endsWith('.nsbmd'))
          .slice(0, 4)
          .map((f) => f.path);
      else if (a.kind === 'Track')
        files = state.files
          .filter(
            (f) =>
              f.path.includes(`/studio${a.options.course}.carc/`) ||
              f.path.includes(`/studio${a.options.course}Tex.carc/`),
          )
          .map((f) => f.path);
      else if (a.kind === 'Cup') {
        state.wizard_catalog.cups[a.slot] = a.courses;
        files = ['system/arm9.bin'];
      } else if (a.kind === 'KartStats') {
        state.wizard_catalog.karts[a.slot] = {
          id: a.slot,
          speed: a.speed,
          acceleration: a.acceleration,
        };
        files = ['data/KartModelMainA.carc/kartphysicalparam.bin'];
      } else if (a.kind === 'Music') {
        files = ['data/Sound/sound_data.sdat', 'system/arm9.bin'];
        state.song_report = { ram_bytes: 8192, route: 'sampled', loop_sample: 0 };
      } else if (a.kind === 'Icon' || a.kind === 'Portrait') files = [a.tiles];
      else if (a.kind === 'Name') {
        const name = state.names.find((n) => n.path === a.path && n.index === a.index);
        if (name) name.text = a.text;
        files = [a.path];
      } else if (a.kind === 'Table') files = ['system/arm9.bin'];
      else if (a.kind === 'SetWheels')
        files = [
          'data/KartModelMainA.carc/kartoffsetdata.bin',
          'data/KartModelMenu.carc/kartoffsetdata.bin',
        ];
      else files = [a.path];
      for (const f of files) if (!state.edited.includes(f)) state.edited.push(f);
      state.wizard_report = {
        files,
        polygons: 6,
        warnings:
          a.kind === 'Track'
            ? ['Generated track files checked against synthetic size limits.']
            : [],
      };
      break;
    }
    case 'ModelMaterial': {
      const info = await scene(r.path),
        detail = info.material_details[r.material];
      if (r.action.kind === 'Recolour') detail.colors[r.action.index] = r.action.color;
      sceneBank[r.path] = info;
      editedPath = r.path;
      title = 'Edit model texture';
      break;
    }
    case 'ModelBatch': {
      const info = await scene(r.path);
      const targets =
        r.linked && info.linked_copies.length ? info.linked_copies.map((c) => c.path) : [r.path];
      for (const path of targets) {
        const data = await binary({
          path,
          mode: 'glb',
          palette: null,
          screen: null,
          texture: 0,
          bank: 0,
          start: 0,
          length: 0,
        });
        const gltf = await new GLTFLoader().parseAsync(data, '');
        const metadata = await scene(path);
        const meshes: THREE.Mesh[] = [];
        gltf.scene.traverse((o) => {
          if (o instanceof THREE.Mesh && typeof o.userData.part === 'number') meshes.push(o);
        });
        for (const action of r.actions) {
          title =
            action.kind === 'Edit'
              ? {
                  Transform: 'Transform model',
                  Extrude: 'Extrude selection',
                  Inset: 'Inset selection',
                  Delete: 'Delete model selection',
                  Merge: 'Merge selected vertices',
                }[action.operation.kind]
              : action.kind + ' model';
          if (action.kind === 'Import') continue;
          const mesh = meshes.find((m) => m.userData.part === action.part);
          if (!mesh) continue;
          if (action.kind === 'Edit')
            mesh.geometry = previewEdit(mesh.geometry, action.selection, action.operation);
          if (action.kind === 'Decimate')
            mesh.geometry.setIndex(
              Array.from(mesh.geometry.index?.array ?? []).slice(0, action.target * 3),
            );
          const part = metadata.parts[action.part];
          if (part)
            part.triangles =
              (mesh.geometry.index?.count ?? mesh.geometry.getAttribute('position').count) / 3;
        }
        metadata.budget.polygons = metadata.parts.reduce((n, p) => n + p.triangles, 0);
        sceneBank[path] = metadata;
        modelBytes[path] = (await new GLTFExporter().parseAsync(gltf.scene, {
          binary: true,
          animations: gltf.animations,
        })) as ArrayBuffer;
      }
      for (const metadata of Object.values(sceneBank))
        for (const copy of metadata.linked_copies)
          if (sceneBank[copy.path]) copy.triangles = sceneBank[copy.path].budget.polygons;
      if (!sceneBank[r.path]?.path.includes('/KartModel')) sceneState = clone(sceneBank[r.path]);
      changePaths = targets;
      editedPath = r.path;
      break;
    }
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
        bank: null,
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
    case 'Save':
      break;
    case 'SaveAs':
      state.project_path = r.path;
      break;
    case 'Snapshot':
      break;
    case 'Select':
      if (!state.files.some((f) => f.path === r.path)) throw new Error('File not found');
      state.selected = r.path;
      break;
    case 'TexturePalette':
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
        sceneBank = clone(h.oldSceneBank);
        modelBytes = clone(h.oldModelBytes);
        state.audio_resources = clone(h.oldAudio);
        colors = clone(h.before);
        Object.assign(messages, clone(h.oldMessages));
        state.edited = clone(h.oldEdited);
        state.manifest = clone(h.oldManifest);
        state.names = clone(h.oldNames);
        state.wizard_catalog = clone(h.oldCatalog);
        state.wizard_report = null;
      }
      break;
    case 'Redo':
      if (cursor < history.length) {
        const h = history[cursor++];
        pixels = clone(h.newPixels);
        sceneState = clone(h.newScene);
        sceneBank = clone(h.newSceneBank);
        modelBytes = clone(h.newModelBytes);
        state.audio_resources = clone(h.newAudio);
        colors = clone(h.after);
        Object.assign(messages, clone(h.newMessages));
        state.edited = clone(h.newEdited);
        state.manifest = clone(h.newManifest);
        state.names = clone(h.newNames);
        state.wizard_catalog = clone(h.newCatalog);
        state.wizard_report = null;
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
  if (editedPath && !changePaths.length) changePaths = [editedPath];
  for (const path of changePaths) if (!state.edited.includes(path)) state.edited.push(path);
  if (title) {
    history = history.slice(0, cursor);
    history.push({
      oldSceneBank,
      newSceneBank: clone(sceneBank),
      oldModelBytes,
      newModelBytes: clone(modelBytes),
      paths: changePaths,
      oldCatalog,
      newCatalog: clone(state.wizard_catalog),
      oldNames,
      newNames: clone(state.names),
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
  const used = state.selected.replace(/^data\/KartModelMenu\.carc\//, 'data/KartModelMenu/');
  state.warnings =
    used !== state.selected && state.files.some((f) => f.path === used)
      ? [
          `${state.selected} is a stale duplicate. The game uses ${used}; edit the loose file instead.`,
        ]
      : [];
  return clone(state);
}
export async function binary(r: BinaryRequest): Promise<ArrayBuffer> {
  if (r.mode === 'glb' && modelBytes[r.path]) return modelBytes[r.path].slice(0);
  if (r.mode === 'audio-wav') return (await fetch('/demo/tone.wav')).arrayBuffer();
  if (r.mode === 'glb' && !r.path.includes('/Course/')) {
    if (r.path === 'data/Studio/editing.nsbmd')
      return (await fetch('/demo/editing.glb')).arrayBuffer();
    const animated = state.pairings.find((p) => p.path === r.path)?.animations.length;
    return (await fetch(`/demo/${animated ? 'driver' : 'static'}.glb`)).arrayBuffer();
  }
  if (r.mode === 'glb' || r.mode === 'course-glb' || r.mode === 'seated-reference')
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
    if (r.path === 'data/Studio/editing.nsbmd') {
      const bitmap = await createImageBitmap(await (await fetch('/demo/editing.png')).blob());
      const canvas = document.createElement('canvas');
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      const ctx = canvas.getContext('2d')!;
      ctx.drawImage(bitmap, 0, 0);
      bitmap.close();
      const pixels = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const original = (wizardData.scenes as unknown as Record<string, SceneInfo>)[r.path]
        .material_details[0].colors;
      const palette = sceneBank[r.path].material_details[0].colors;
      const rgb = (c: number) => rgb555(c).map((c) => (c << 3) | (c >> 2));
      for (let i = 0; i < pixels.data.length; i += 4) {
        const index = original.findIndex((c) =>
          rgb(c).every((v, k) => Math.abs(v - pixels.data[i + k]) < 3),
        );
        if (index >= 0) pixels.data.set(rgb(palette[index]), i);
      }
      ctx.putImageData(pixels, 0, 0);
      return (
        await new Promise<Blob>((resolve) => canvas.toBlob((b) => resolve(b!), 'image/png'))
      ).arrayBuffer();
    }
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

import { pickDirectory, pickRom, savePath } from './backend';
import { stages, useStore, workspaces, type Workspace } from './store';
import type { Snapshot } from './bindings';
import { useView } from './view';
import { useScene } from './scene-view';
export interface Command {
  id: string;
  title: string;
  category: string;
  shortcut?: string;
  when: (s: Snapshot | null) => boolean;
  run: () => void | Promise<void>;
}
const always = () => true;
const project = (s: Snapshot | null) => !!s?.manifest;
export function openWorkspace(workspace: Workspace) {
  const store = useStore.getState();
  store.set({ workspace });
  const formats =
    workspace === 'Text'
      ? ['bmg']
      : workspace === 'Texture & 2D'
        ? ['nclr', 'ncgr', 'nscr', 'nsbtx', 'ncer', 'nanr']
        : workspace === 'Model' || workspace === 'Rig & Animate'
          ? ['nsbmd']
          : workspace === 'Track'
            ? ['nkm', 'kcl', 'nsbmd']
            : [];
  if (
    formats.length &&
    !store.snapshot?.files.find(
      (f) => f.path === store.snapshot?.selected && formats.includes(f.format),
    )
  ) {
    const file = store.snapshot?.files.find((f) => f.format === formats[0]);
    if (file) void store.run({ kind: 'Select', path: file.path });
  }
}
export async function openFile(workspace: Workspace, path: string) {
  const store = useStore.getState();
  await store.run({ kind: 'Select', path });
  if (useStore.getState().snapshot?.selected === path) openWorkspace(workspace);
}
export async function exportSelection(to: string) {
  const { snapshot, run } = useStore.getState();
  const view = useView.getState();
  if (!snapshot?.selected) return;
  const name = snapshot.selected.split('/').pop() ?? 'file';
  const output = await savePath(to === 'raw' ? name : `${name}.${to}`);
  if (!output) return;
  if (to === 'raw') await run({ kind: 'Extract', path: snapshot.selected, output });
  else
    await run({
      kind: 'Export',
      path: snapshot.selected,
      output,
      to,
      palette: view.palette || null,
      screen: view.screen || null,
      texture: view.texture,
      bank: view.bank,
    });
}
export const commands: Command[] = [
  { id: 'project.recipes', title: 'Beginner recipes?', category: 'Project', when: project, run: () => useStore.getState().set({ wizardOpen: true }) },
  {
    id: 'file.exportDs',
    title: 'Export for DS…',
    category: 'File',
    when: project,
    run: () => useStore.getState().set({ exportOpen: true }),
  },
  {
    id: 'project.reload',
    title: 'Reload Test Bench',
    category: 'Project',
    shortcut: 'Ctrl+R',
    when: (s) => project(s) && useStore.getState().workspace === 'Test Bench',
    run: () => useStore.getState().run({ kind: 'Reload', game: useScene.getState().game }),
  },
  {
    id: 'project.play',
    title: 'Play project',
    category: 'Project',
    shortcut: 'F5',
    when: project,
    run: async () => {
      openWorkspace('Test Bench');
      await useStore.getState().run({ kind: 'Play', game: useScene.getState().game });
    },
  },
  {
    id: 'project.stop',
    title: 'Stop game',
    category: 'Project',
    shortcut: 'Shift+F5',
    when: (s) => !!s?.bench.running,
    run: () => useStore.getState().run({ kind: 'Stop' }),
  },
  ...(['glb', 'obj'] as const).map((to): Command => ({
    id: `file.${to}`,
    title: `Export as ${to.toUpperCase()}…`,
    category: 'File',
    when: (s) =>
      !!s?.files.find(
        (f) => f.path === s.selected && f.format === (to === 'glb' ? 'nsbmd' : 'kcl'),
      ),
    run: () => exportSelection(to),
  })),
  {
    id: 'file.openRom',
    title: 'Open ROM…',
    category: 'File',
    shortcut: 'Ctrl+O',
    when: always,
    run: async () => {
      const path = await pickRom();
      if (path) await useStore.getState().run({ kind: 'OpenRom', path });
    },
  },
  {
    id: 'project.create',
    title: 'Create project…',
    category: 'Project',
    when: (s) => !!s?.rom_path,
    run: () => useStore.getState().set({ createOpen: true }),
  },
  {
    id: 'project.open',
    title: 'Open project…',
    category: 'File',
    when: always,
    run: async () => {
      const path = await pickDirectory();
      if (path) await useStore.getState().run({ kind: 'OpenProject', path });
    },
  },
  {
    id: 'edit.undo',
    title: 'Undo',
    category: 'Edit',
    shortcut: 'Ctrl+Z',
    when: (s) => !!s?.can_undo,
    run: () => useStore.getState().run({ kind: 'Undo' }),
  },
  {
    id: 'edit.redo',
    title: 'Redo',
    category: 'Edit',
    shortcut: 'Ctrl+Shift+Z',
    when: (s) => !!s?.can_redo,
    run: () => useStore.getState().run({ kind: 'Redo' }),
  },
  {
    id: 'project.pack',
    title: 'Pack mod',
    category: 'Project',
    shortcut: 'Ctrl+Shift+B',
    when: project,
    run: () => useStore.getState().run({ kind: 'Pack' }),
  },
  {
    id: 'project.validate',
    title: 'Validate project',
    category: 'Project',
    when: project,
    run: () => useStore.getState().run({ kind: 'Validate' }),
  },
  {
    id: 'file.extract',
    title: 'Extract selection…',
    category: 'File',
    when: (s) => !!s?.selected,
    run: () => exportSelection('raw'),
  },
  {
    id: 'file.png',
    title: 'Export as PNG…',
    category: 'File',
    when: (s) => !!s?.selected,
    run: () => exportSelection('png'),
  },
  {
    id: 'file.json',
    title: 'Export as JSON…',
    category: 'File',
    when: (s) => !!s?.selected,
    run: () => exportSelection('json'),
  },
  {
    id: 'view.palette',
    title: 'Command palette',
    category: 'View',
    shortcut: 'Ctrl+Shift+P',
    when: always,
    run: () => useStore.getState().set({ paletteOpen: true }),
  },
  {
    id: 'view.settings',
    title: 'Settings & keybindings',
    category: 'Edit',
    shortcut: 'Ctrl+,',
    when: always,
    run: () => useStore.getState().set({ settingsOpen: true }),
  },
  {
    id: 'view.reset',
    title: 'Reset workspace layout',
    category: 'View',
    when: always,
    run: () => {
      const s = useStore.getState();
      localStorage.removeItem(`layout:${s.workspace}`);
      s.set({ layoutVersion: s.layoutVersion + 1 });
    },
  },
  ...workspaces
    .filter((w) => !stages[w])
    .map((w): Command => ({
      id: `workspace.${w}`,
      title: `Go to ${w}`,
      category: 'Workspace',
      when: always,
      run: () => openWorkspace(w),
    })),
];
export function fuzzyScore(query: string, title: string): number {
  let score = 0,
    pos = 0;
  const text = title.toLowerCase();
  for (const c of query.toLowerCase().replaceAll(' ', '')) {
    const found = text.indexOf(c, pos);
    if (found < 0) return -1;
    score += found === pos ? 8 : 1;
    pos = found + 1;
  }
  return score;
}
export function matchesKey(e: KeyboardEvent, binding: string): boolean {
  const parts = binding.toLowerCase().split('+');
  const key = parts.pop();
  return (
    (e.ctrlKey || e.metaKey) === parts.includes('ctrl') &&
    e.shiftKey === parts.includes('shift') &&
    e.altKey === parts.includes('alt') &&
    e.key.toLowerCase() === key
  );
}
export function invokeCommand(command: Command) {
  if (command.when(useStore.getState().snapshot))
    Promise.resolve(command.run()).catch((e) => useStore.getState().set({ error: String(e) }));
}

import { create } from 'zustand';
import type { Request, Snapshot } from './bindings';
import { dispatch } from './backend';
import { useView } from './view';
export const workspaces = [
  'Home',
  'Game browser',
  'Image gallery',
  'Overview',
  'Explorer',
  'Texture & 2D',
  'Text',
  'Track',
  'Model',
  'Rig & Animate',
  'Audio',
  'Code',
  'Test Bench',
] as const;
export type Workspace = (typeof workspaces)[number];
export const stages: Partial<Record<Workspace, number>> = {
  Code: 7,
};
export type Recipe =
  'Character' | 'Kart' | 'Track' | 'Music' | 'Cups' | 'Items' | 'Names' | 'Tables';
export function storedList(key: string): string[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(key) ?? '[]');
    return Array.isArray(value)
      ? value.filter((v): v is string => typeof v === 'string').slice(0, 20)
      : [];
  } catch {
    return [];
  }
}
function storedMode() {
  try {
    return localStorage.getItem('toolbox-mode');
  } catch {
    return null;
  }
}
export function remember(key: string, value: string) {
  const list = [value, ...storedList(key).filter((v) => v !== value)].slice(0, 20);
  try {
    localStorage.setItem(key, JSON.stringify(list));
  } catch {
    /* Storage is optional. */
  }
}
interface State {
  previewPath: string;
  textIndex: number;
  advanced: boolean;
  recipe: Recipe;
  drafts: Record<string, Request>;
  opened: string[];
  navigation: string[];
  navigationIndex: number;
  recentVersion: number;
  snapshot: Snapshot | null;
  busy: boolean;
  error: string;
  notice: string;
  workspace: Workspace;
  settingsOpen: boolean;
  exportOpen: boolean;
  createOpen: boolean;
  paletteOpen: boolean;
  wizardOpen: boolean;
  layoutVersion: number;
  set: (state: Partial<Omit<State, 'set' | 'run'>>) => void;
  run: (request: Request) => Promise<void>;
}
let queue = Promise.resolve();
const query = new URLSearchParams(location.search).get('workspace');
export const useStore = create<State>((set, get) => ({
  previewPath: '',
  textIndex: 0,
  // Simple (beginner pages) or Studio (every editor). Remembered between sessions.
  advanced: query
    ? !['Home', 'Game browser', 'Image gallery'].includes(query)
    : storedMode() === 'studio',
  recipe: 'Character',
  drafts: {},
  opened: [],
  navigation: [],
  navigationIndex: -1,
  recentVersion: 0,
  snapshot: null,
  busy: false,
  error: '',
  notice: 'Ready',
  workspace: workspaces.find((w) => w === query) ?? 'Home',
  settingsOpen: false,
  exportOpen: false,
  createOpen: false,
  paletteOpen: false,
  wizardOpen: false,
  layoutVersion: 0,
  set,
  run: (request) => {
    const operation = queue.then(async () => {
      set({ busy: true, error: '' });
      try {
        const snapshot = await dispatch(request);
        if (request.kind === 'OpenRom' || request.kind === 'OpenProject') {
          useView.setState({
            palette: '',
            tiles: '',
            screen: '',
            texture: 0,
            bank: 0,
            selectedColors: [0],
          });
        }
        if (
          request.kind === 'OpenRom' ||
          request.kind === 'OpenProject' ||
          request.kind === 'CreateProject' ||
          request.kind === 'CreateFromBase'
        ) {
          set({ workspace: 'Home', drafts: {}, opened: [], navigation: [], navigationIndex: -1 });
        }
        if (
          ['OpenProject', 'CreateProject', 'CreateFromBase', 'SaveAs'].includes(request.kind) &&
          snapshot.project_path
        )
          remember('recent-projects', snapshot.project_path);
        if (request.kind === 'Select' && snapshot.selected === request.path) {
          const state = get();
          const navigation =
            state.navigation[state.navigationIndex] === request.path
              ? state.navigation
              : [...state.navigation.slice(0, state.navigationIndex + 1), request.path];
          set({
            textIndex: 0,
            opened: [...state.opened.filter((p) => p !== request.path), request.path],
            navigation,
            navigationIndex:
              navigation.length === state.navigation.length && navigation === state.navigation
                ? state.navigationIndex
                : navigation.length - 1,
          });
          remember('recent:' + snapshot.rom_hash, request.path);
        }
        set({
          snapshot,
          recentVersion: get().recentVersion + 1,
          notice:
            request.kind === 'Snapshot' || request.kind === 'Select'
              ? 'Ready'
              : `${request.kind} completed`,
        });
      } catch (error) {
        set({ error: String(error), notice: 'Action failed' });
      } finally {
        set({ busy: false });
      }
    });
    queue = operation.catch(() => undefined);
    return operation;
  },
}));
export function select(path: string) {
  void useStore.getState().run({ kind: 'Select', path });
}

export function draft(key: string, request: Request | null) {
  const store = useStore.getState();
  const drafts = { ...store.drafts };
  if (request) drafts[key] = request;
  else delete drafts[key];
  store.set({ drafts });
}
export async function saveProject() {
  const store = useStore.getState();
  for (const [key, request] of Object.entries(store.drafts)) {
    await store.run(request);
    if (useStore.getState().error) return;
    if (useStore.getState().drafts[key] === request) draft(key, null);
  }
  await store.run({ kind: 'Save' });
}

useStore.subscribe((s, before) => {
  if (s.advanced === before.advanced) return;
  try {
    localStorage.setItem('toolbox-mode', s.advanced ? 'studio' : 'simple');
  } catch {
    /* Storage is optional. */
  }
});

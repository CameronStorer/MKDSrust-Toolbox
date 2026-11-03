import { create } from 'zustand';
import type { Request, Snapshot } from './bindings';
import { dispatch } from './backend';
import { useView } from './view';
export const workspaces = [
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
  'Rig & Animate': 5,
  Audio: 8,
  Code: 7,
};
interface State {
  snapshot: Snapshot | null;
  busy: boolean;
  error: string;
  notice: string;
  workspace: Workspace;
  settingsOpen: boolean;
  paletteOpen: boolean;
  layoutVersion: number;
  set: (state: Partial<Omit<State, 'set' | 'run'>>) => void;
  run: (request: Request) => Promise<void>;
}
let queue = Promise.resolve();
const query = new URLSearchParams(location.search).get('workspace');
export const useStore = create<State>((set) => ({
  snapshot: null,
  busy: false,
  error: '',
  notice: 'Ready',
  workspace: workspaces.find((w) => w === query) ?? 'Overview',
  settingsOpen: false,
  paletteOpen: false,
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
        set({
          snapshot,
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

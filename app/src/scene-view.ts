import { create } from 'zustand';
import type { SceneInfo } from './bindings';
export const layerDefaults = {
  KCL: { name: 'Collision', color: '#a4c38b', visible: true, locked: false },
  EPOI: { name: 'CPU routes', color: '#ffae69', visible: true, locked: false },
  MEPO: { name: 'Battle CPU routes', color: '#ffd46a', visible: true, locked: false },
  CPOI: { name: 'Checkpoints', color: '#76c5ef', visible: true, locked: false },
  IPOI: { name: 'Item routes', color: '#c5a0ef', visible: true, locked: false },
  KTPJ: { name: 'Respawns', color: '#f08293', visible: true, locked: false },
  KTPS: { name: 'Start positions', color: '#a6dd97', visible: true, locked: false },
  OBJI: { name: 'Objects', color: '#f0c267', visible: true, locked: false },
  AREA: { name: 'Areas', color: '#84c9bd', visible: true, locked: false },
  CAME: { name: 'Cameras', color: '#9faee8', visible: true, locked: false },
};
export type Layer = keyof typeof layerDefaults;
interface SceneState {
  part: number;
  triangle: number;
  gizmo: 'off' | 'translate' | 'rotate' | 'scale';
  brush: boolean;
  attribute: number;
  info: SceneInfo | null;
  selected: string;
  layers: typeof layerDefaults;
  mode: 'textured' | 'solid' | 'wireframe';
  grid: boolean;
  animation: number;
  frame: number;
  playing: boolean;
  frameRequest: number;
  game: string;
  set: (state: Partial<Omit<SceneState, 'set'>>) => void;
}
export const useScene = create<SceneState>((set) => ({
  part: 0,
  triangle: 0,
  gizmo: 'off',
  brush: false,
  attribute: 0x8000,
  info: null,
  selected: '',
  layers: structuredClone(layerDefaults),
  mode: 'textured',
  grid: true,
  animation: -1,
  frame: 0,
  playing: false,
  frameRequest: 0,
  game: '',
  set,
}));

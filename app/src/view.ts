import { create } from 'zustand';
interface View {
  palette: string;
  tiles: string;
  screen: string;
  texture: number;
  bank: number;
  zoom: number;
  grid: boolean;
  selectedColors: number[];
  set: (v: Partial<Omit<View, 'set'>>) => void;
}
export const useView = create<View>((set) => ({
  palette: '',
  tiles: '',
  screen: '',
  texture: 0,
  bank: 0,
  zoom: 3,
  grid: false,
  selectedColors: [0],
  set,
}));

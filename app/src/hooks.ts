import { useEffect, useState } from 'react';
import { inspect } from './backend';
import { useStore } from './store';
import type { Preview } from './bindings';
export const emptyPreview: Preview = {
  cells: [],
  sequences: [],
  colors: [],
  palette_stride: 0,
  messages: [],
  textures: [],
  palettes: [],
  width: 0,
  height: 0,
};
export function usePreview(path: string) {
  const snapshot = useStore((s) => s.snapshot);
  const [preview, setPreview] = useState<Preview>(emptyPreview);
  useEffect(() => {
    if (!path) {
      setPreview(emptyPreview);
      return;
    }
    if (snapshot?.selected === path) {
      setPreview(snapshot.preview);
      return;
    }
    let active = true;
    void inspect(path)
      .then((p) => {
        if (active) setPreview(p);
      })
      .catch(() => {
        if (active) setPreview(emptyPreview);
      });
    return () => {
      active = false;
    };
  }, [path, snapshot]);
  return preview;
}
export function companion(files: { path: string; format: string }[], path: string, format: string) {
  const entries = useStore.getState().snapshot?.game_entries ?? [];
  const known = entries.find((e) => e.path === path);
  if (format === 'nclr' && known?.palette) return known.palette;
  if (format === 'nscr' && known?.screen) return known.screen;
  const stem = path.replace(/\.[^/.]+$/, '').toLowerCase();
  return (
    files.find((f) => f.format === format && f.path.replace(/\.[^/.]+$/, '').toLowerCase() === stem)
      ?.path ?? ''
  );
}

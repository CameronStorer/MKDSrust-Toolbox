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
  const stem = path.replace(/\.[^/.]+$/, '');
  return (
    files.find((f) => f.path === `${stem}.${format}`)?.path ??
    files.find(
      (f) =>
        f.format === format &&
        f.path.slice(0, f.path.lastIndexOf('/')) === path.slice(0, path.lastIndexOf('/')),
    )?.path ??
    ''
  );
}

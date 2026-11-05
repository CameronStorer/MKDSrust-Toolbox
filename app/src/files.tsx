import { useStore } from './store';
import { openFile, exportSelection } from './commands';
import { Hex } from './panels/Hex';

export const fileTypes: Record<string, [string, string]> = {
  carc: ['Archive', '▣'], narc: ['Archive', '▣'],
  ncgr: ['Image', '▧'], nscr: ['Image', '▧'], ncer: ['Image', '▧'],
  nclr: ['Palette', '◉'], nsbmd: ['Model', '◇'], nsbtx: ['Texture', '▧'],
  nsbca: ['Animation', '▷'], nsbta: ['Animation', '▷'], nsbtp: ['Animation', '▷'],
  nanr: ['Animation', '▷'], nsbma: ['Animation', '▷'], nsbva: ['Animation', '▷'],
  kcl: ['Collision', '△'], nkm: ['Track layout', '⌖'], bmg: ['Text', 'T'],
  sdat: ['Audio', '♫'], sseq: ['Audio', '♫'], swav: ['Audio', '♫'],
  strm: ['Audio', '♫'], bin: ['Data / code', '≡'],
};
export function otherLanguage(path: string) {
  return /_(es|fr|ge|it)([./]|$)/i.test(path);
}
export function openAsset(path: string) {
  const store = useStore.getState();
  const format = store.snapshot?.files.find((f) => f.path === path)?.format;
  store.set({ advanced: true });
  const workspace = format === 'bmg' ? 'Text'
    : format === 'nsbmd' || format === 'nsbca' ? 'Model'
    : ['nclr', 'ncgr', 'nscr', 'nsbtx', 'ncer', 'nanr'].includes(format ?? '') ? 'Texture & 2D'
    : ['nkm', 'kcl'].includes(format ?? '') ? 'Track'
    : format === 'sdat' ? 'Audio' : 'Explorer';
  void openFile(workspace, path).then(() => {
    if (workspace === 'Explorer') store.set({ previewPath: path });
  });
}
export function FileLink({ path, children }: { path: string; children?: React.ReactNode }) {
  return <button className="file-link" title={path} onClick={() => openAsset(path)}>{children ?? path}</button>;
}
export function FilePreview() {
  const path = useStore((s) => s.previewPath);
  if (!path) return null;
  return <div className="modal-backdrop"><section className="dialog" role="dialog" aria-label="File preview">
    <h2>{path.split('/').pop()}</h2><code>{path}</code>
    <div style={{ height: 320 }}><Hex /></div>
    <button onClick={() => void exportSelection('raw')}>Export file…</button>
    <button onClick={() => useStore.getState().set({ previewPath: '' })}>Close preview</button>
  </section></div>;
}

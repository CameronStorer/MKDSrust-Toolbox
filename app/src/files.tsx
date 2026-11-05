import { useStore } from './store';
import { openFile, exportSelection } from './commands';
import { Hex } from './panels/Hex';

import { revealFile } from './explorer-view';
export { fileTypes, otherLanguage } from './file-types';
export function LinkedText({ text }: { text: string }) {
  const files = useStore((s) => s.snapshot?.files ?? []);
  const paths = files
    .map((f) => f.path)
    .filter((p) => text.includes(p))
    .sort((a, b) => b.length - a.length);
  const parts: React.ReactNode[] = [];
  let rest = text;
  while (rest) {
    const next = paths
      .map((p) => ({ p, at: rest.indexOf(p) }))
      .filter((v) => v.at >= 0)
      .sort((a, b) => a.at - b.at || b.p.length - a.p.length)[0];
    if (!next) {
      parts.push(rest);
      break;
    }
    parts.push(rest.slice(0, next.at), <FileLink key={parts.length} path={next.p} />);
    rest = rest.slice(next.at + next.p.length);
  }
  return <>{parts}</>;
}
export function openAsset(path: string) {
  const store = useStore.getState();
  const used = path.replace(/^data\/KartModelMenu\.carc\//, 'data/KartModelMenu/');
  if (used !== path && store.snapshot?.files.some((f) => f.path === used)) {
    store.set({
      notice: 'Opened the loose menu file used by the game. The archive copy is stale.',
    });
    path = used;
  }
  const format = store.snapshot?.files.find((f) => f.path === path)?.format;
  revealFile(path);
  store.set({ advanced: true, wizardOpen: false, exportOpen: false });
  const workspace =
    format === 'bmg'
      ? 'Text'
      : format === 'nsbmd' || format === 'nsbca'
        ? 'Model'
        : ['nclr', 'ncgr', 'nscr', 'nsbtx', 'ncer', 'nanr'].includes(format ?? '')
          ? 'Texture & 2D'
          : ['nkm', 'kcl'].includes(format ?? '')
            ? 'Track'
            : format === 'sdat'
              ? 'Audio'
              : 'Explorer';
  void openFile(workspace, path).then(() => {
    if (workspace === 'Explorer') store.set({ previewPath: path });
  });
}
export function FileLink({ path, children }: { path: string; children?: React.ReactNode }) {
  return (
    <button className="file-link" title={path} onClick={() => openAsset(path)}>
      {children ?? path}
    </button>
  );
}
export function FilePreview() {
  const path = useStore((s) => s.previewPath);
  if (!path) return null;
  return (
    <div className="modal-backdrop">
      <section className="dialog" role="dialog" aria-label="File preview">
        <h2>{path.split('/').pop()}</h2>
        <code>{path}</code>
        <div style={{ height: 320 }}>
          <Hex />
        </div>
        <button onClick={() => void exportSelection('raw')}>Export file…</button>
        <button onClick={() => useStore.getState().set({ previewPath: '' })}>Close preview</button>
      </section>
    </div>
  );
}

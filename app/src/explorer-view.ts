import { create } from 'zustand';
import type { FileNode } from './bindings';
import { fileTypes, otherLanguage } from './file-types';

interface ExplorerView {
  query: string;
  closed: string[];
  languages: boolean;
  grouped: boolean;
  scroll: number;
  selection: string;
  reveal: string;
}
export const useExplorer = create<ExplorerView>(() => ({
  query: '',
  closed: [],
  languages: false,
  grouped: true,
  scroll: 0,
  selection: '',
  reveal: '',
}));
export function revealFile(path: string) {
  useExplorer.setState((s) => ({
    query: '',
    selection: path,
    reveal: path,
    languages: s.languages || otherLanguage(path),
    closed: s.closed.filter((p) => !path.startsWith(p + '/')),
  }));
}
export interface TreeRow {
  path: string;
  name: string;
  depth: number;
  folder: boolean;
  file?: FileNode;
  hint?: string;
}
const order = [
  'Archive',
  'Image',
  'Palette',
  'Model',
  'Texture',
  'Animation',
  'Collision',
  'Track layout',
  'Text',
  'Audio',
  'Code',
  'Data',
];
export function fileType(file?: FileNode) {
  return fileTypes[file?.format ?? ''] ?? ['Data', '≡'];
}
export function treeRows(
  files: FileNode[],
  view: Pick<ExplorerView, 'query' | 'closed' | 'languages' | 'grouped'>,
) {
  const all = new Map<string, TreeRow>();
  for (const file of files) {
    if (!view.languages && otherLanguage(file.path)) continue;
    const parts = file.path.split('/');
    parts.forEach((name, i) => {
      const path = parts.slice(0, i + 1).join('/');
      if (!all.has(path)) all.set(path, { path, name, depth: i, folder: true });
    });
    all.set(file.path, {
      path: file.path,
      name: file.name,
      depth: parts.length - 1,
      folder: file.archive,
      file,
    });
  }
  const children = new Map<string, TreeRow[]>();
  for (const row of all.values()) {
    const parent = row.path.slice(0, row.path.lastIndexOf('/'));
    const key = row.depth ? parent : '';
    children.set(key, [...(children.get(key) ?? []), row]);
  }
  const query = view.query.toLowerCase();
  const matched = new Set<string>();
  for (const row of all.values()) {
    if (!row.path.toLowerCase().includes(query)) continue;
    const parts = row.path.split('/');
    parts.forEach((_, i) => matched.add(parts.slice(0, i + 1).join('/')));
  }
  const result: TreeRow[] = [];
  const visit = (parent: string) => {
    const rows = (children.get(parent) ?? []).sort((a, b) => {
      const kind = (r: TreeRow) => (r.folder && !r.file ? -1 : order.indexOf(fileType(r.file)[0]));
      return (view.grouped ? kind(a) - kind(b) : 0) || a.name.localeCompare(b.name);
    });
    for (const row of rows) {
      if (query && !matched.has(row.path)) continue;
      if (row.folder) {
        const descendants = files.filter(
          (f) => f.path.startsWith(row.path + '/') && (view.languages || !otherLanguage(f.path)),
        );
        const counts = new Map<string, number>();
        descendants
          .filter((f) => !f.archive)
          .forEach((f) => {
            const type = fileType(f)[0].toLowerCase();
            counts.set(type, (counts.get(type) ?? 0) + 1);
          });
        row.hint = [...counts].map(([type, n]) => `${n} ${type}${n === 1 ? '' : 's'}`).join(', ');
      }
      result.push(row);
      if (row.folder && (query || !view.closed.includes(row.path))) visit(row.path);
    }
  };
  visit('');
  return result;
}

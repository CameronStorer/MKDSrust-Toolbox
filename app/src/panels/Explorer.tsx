import { useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import {
  ChevronDown,
  ChevronRight,
  Folder,
  File,
  Archive,
  Palette,
  Type,
  Image,
  Search,
  Download,
  ExternalLink,
} from 'lucide-react';
import { useStore, select } from '../store';
import { exportSelection, openWorkspace } from '../commands';
import type { FileNode } from '../bindings';
interface TreeRow {
  path: string;
  name: string;
  depth: number;
  folder: boolean;
  file?: FileNode;
}
export function Explorer() {
  const snapshot = useStore((s) => s.snapshot);
  const [query, setQuery] = useState('');
  const [closed, setClosed] = useState(new Set<string>());
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  const rows = useMemo(() => {
    const all = new Map<string, TreeRow>();
    for (const file of snapshot?.files ?? []) {
      const parts = file.path.split('/');
      parts.forEach((name, i) => {
        const path = parts.slice(0, i + 1).join('/');
        if (!all.has(path)) all.set(path, { path, name, depth: i, folder: i < parts.length - 1 });
      });
      all.set(file.path, {
        path: file.path,
        name: file.name,
        depth: parts.length - 1,
        folder: file.archive,
        file,
      });
    }
    return [...all.values()]
      .sort((a, b) => a.path.localeCompare(b.path))
      .filter((row) => {
        if (query) return row.path.toLowerCase().includes(query.toLowerCase());
        const parts = row.path.split('/');
        return !parts.some(
          (_, i) => i < parts.length - 1 && closed.has(parts.slice(0, i + 1).join('/')),
        );
      });
  }, [snapshot?.files, query, closed]);
  const virtual = useVirtualizer({
    count: rows.length,
    getScrollElement: () => ref.current,
    estimateSize: () => 27,
    overscan: 12,
  });
  const toggle = (path: string) =>
    setClosed((old) => {
      const s = new Set(old);
      if (s.has(path)) s.delete(path);
      else s.add(path);
      return s;
    });
  return (
    <div className="panel explorer" onClick={() => setMenu(null)}>
      <div className="search">
        <Search size={13} />
        <input
          aria-label="Search files"
          placeholder="Filter files…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <kbd>/</kbd>
      </div>
      <div className="tree-source">
        <span className="dot" /> ROM FILESYSTEM <span>{snapshot?.files.length ?? 0}</span>
      </div>
      <div ref={ref} className="tree-scroll" role="tree" aria-label="ROM files">
        <div style={{ height: virtual.getTotalSize(), position: 'relative' }}>
          {virtual.getVirtualItems().map((item) => {
            const row = rows[item.index];
            const Icon = row.folder
              ? row.file?.archive
                ? Archive
                : Folder
              : row.file?.format === 'nclr'
                ? Palette
                : row.file?.format === 'bmg'
                  ? Type
                  : ['ncgr', 'nscr', 'nsbtx'].includes(row.file?.format ?? '')
                    ? Image
                    : File;
            const edited = snapshot?.edited.some(
              (p) => p === row.path || p.startsWith(row.path + '/'),
            );
            return (
              <button
                key={row.path}
                role="treeitem"
                aria-selected={snapshot?.selected === row.path}
                aria-expanded={row.folder ? !closed.has(row.path) : undefined}
                className={`tree-row ${snapshot?.selected === row.path ? 'selected' : ''}`}
                style={{
                  position: 'absolute',
                  top: item.start,
                  height: item.size,
                  paddingLeft: 10 + row.depth * 13,
                }}
                onClick={() => {
                  if (row.folder) toggle(row.path);
                  if (row.file) select(row.path);
                }}
                onDoubleClick={() => {
                  if (row.file?.format === 'bmg') openWorkspace('Text');
                  if (row.file?.format === 'nclr') openWorkspace('Texture & 2D');
                }}
                onKeyDown={(e) => {
                  if (row.folder && ['ArrowLeft', 'ArrowRight'].includes(e.key)) {
                    e.preventDefault();
                    toggle(row.path);
                  }
                }}
                onContextMenu={(e) => {
                  e.preventDefault();
                  if (row.file) {
                    select(row.path);
                    setMenu({ x: e.clientX, y: e.clientY });
                  }
                }}
              >
                <span className="chevron">
                  {row.folder ? (
                    closed.has(row.path) ? (
                      <ChevronRight size={12} />
                    ) : (
                      <ChevronDown size={12} />
                    )
                  ) : null}
                </span>
                <Icon size={14} className={row.file?.category.toLowerCase()} />
                <span className="tree-name">{row.name}</span>
                {edited && <span className="edited-dot" title="Edited" />}
                {row.file && !row.folder && (
                  <span className="file-ext">{row.file.format.toUpperCase()}</span>
                )}
              </button>
            );
          })}
        </div>
      </div>
      {!snapshot?.files.length && <div className="empty">Open your ROM to browse its files.</div>}
      <div className="panel-foot">
        {rows.length} entries <span>Read from your ROM</span>
      </div>
      {menu && (
        <div
          className="context-menu"
          style={{
            left: Math.min(menu.x, window.innerWidth - 220),
            top: Math.min(menu.y, window.innerHeight - 150),
          }}
        >
          <button onClick={() => void exportSelection('raw')}>
            <Download size={13} />
            Extract…
          </button>
          <button onClick={() => void exportSelection('png')}>Export as PNG…</button>
          <button onClick={() => void exportSelection('json')}>Export as JSON…</button>
          <button
            onClick={() =>
              openWorkspace(snapshot?.preview.messages.length ? 'Text' : 'Texture & 2D')
            }
          >
            <ExternalLink size={13} />
            Edit in workspace
          </button>
        </div>
      )}
    </div>
  );
}

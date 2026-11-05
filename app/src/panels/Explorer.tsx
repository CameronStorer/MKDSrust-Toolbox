import { useEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import {
  ChevronDown,
  ChevronRight,
  Folder,
  Archive,
  Search,
  Download,
  ExternalLink,
} from 'lucide-react';
import { useStore, select } from '../store';
import { exportSelection, openWorkspace } from '../commands';
import { openAsset } from '../files';
import { fileType, treeRows, useExplorer } from '../explorer-view';
export function Explorer() {
  const snapshot = useStore((s) => s.snapshot);
  const view = useExplorer();
  const { query, closed, languages, grouped } = view;
  const setQuery = (query: string) => useExplorer.setState({ query });
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  const rows = useMemo(
    () => treeRows(snapshot?.files ?? [], { query, closed, languages, grouped }),
    [snapshot?.files, query, closed, languages, grouped],
  );
  const virtual = useVirtualizer({
    count: rows.length,
    getScrollElement: () => ref.current,
    estimateSize: () => 27,
    overscan: 12,
    initialOffset: useExplorer.getState().scroll,
  });
  useEffect(() => {
    if (!view.reveal) return;
    const index = rows.findIndex((r) => r.path === view.reveal);
    if (index >= 0) virtual.scrollToIndex(index, { align: 'auto' });
    useExplorer.setState({ reveal: '' });
  }, [view.reveal, rows, virtual]);
  const toggle = (path: string) =>
    useExplorer.setState({
      closed: closed.includes(path) ? closed.filter((p) => p !== path) : [...closed, path],
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
      <div className="explorer-controls">
        <button onClick={() => useExplorer.setState({ closed: [] })}>Expand all</button>
        <button
          onClick={() =>
            useExplorer.setState({
              closed: treeRows(snapshot?.files ?? [], {
                query: '',
                closed: [],
                languages: true,
                grouped,
              })
                .filter((r) => r.folder)
                .map((r) => r.path),
            })
          }
        >
          Collapse all
        </button>
        <label>
          <input
            type="checkbox"
            checked={grouped}
            onChange={(e) => useExplorer.setState({ grouped: e.target.checked })}
          />
          Group by type
        </label>
        <label>
          <input
            type="checkbox"
            checked={languages}
            onChange={(e) => useExplorer.setState({ languages: e.target.checked })}
          />
          Show other languages
        </label>
      </div>
      <div className="tree-source">
        <span className="dot" /> ROM FILESYSTEM <span>{snapshot?.files.length ?? 0}</span>
      </div>
      <div
        ref={ref}
        className="tree-scroll"
        role="tree"
        aria-label="ROM files"
        onScroll={(e) => useExplorer.setState({ scroll: e.currentTarget.scrollTop })}
      >
        <div style={{ height: virtual.getTotalSize(), position: 'relative' }}>
          {virtual.getVirtualItems().map((item) => {
            const row = rows[item.index];
            const [type, glyph] = fileType(row.file);
            const Icon = row.folder ? (row.file?.archive ? Archive : Folder) : null;
            const edited = snapshot?.edited.some(
              (p) => p === row.path || p.startsWith(row.path + '/'),
            );
            return (
              <button
                key={row.path}
                role="treeitem"
                aria-selected={(view.selection || snapshot?.selected) === row.path}
                aria-expanded={row.folder ? !closed.includes(row.path) : undefined}
                className={`tree-row ${(view.selection || snapshot?.selected) === row.path ? 'selected' : ''}`}
                style={{
                  position: 'absolute',
                  top: item.start,
                  height: item.size,
                  paddingLeft: 10 + row.depth * 13,
                }}
                onClick={() => {
                  useExplorer.setState({ selection: row.path });
                  if (row.folder) toggle(row.path);
                  if (row.file) select(row.path);
                }}
                onDoubleClick={() => {
                  if (row.file) openAsset(row.path);
                }}
                title={row.folder ? row.hint : type + ': ' + row.path}
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
                    closed.includes(row.path) ? (
                      <ChevronRight size={12} />
                    ) : (
                      <ChevronDown size={12} />
                    )
                  ) : null}
                </span>
                {Icon ? (
                  <Icon size={14} />
                ) : (
                  <span className="type-icon" aria-hidden="true">
                    {glyph}
                  </span>
                )}
                <span className="tree-name">{row.name}</span>
                {row.folder && <span className="folder-hint">{row.hint}</span>}
                {edited && <span className="edited-dot" title="Edited" />}
                {row.file && !row.folder && <span className="file-ext">{type}</span>}
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
          {snapshot?.files.find((f) => f.path === snapshot.selected)?.format === 'nsbmd' && (
            <>
              <button onClick={() => openWorkspace('Model')}>Open in Model workspace</button>
              <button onClick={() => void exportSelection('glb')}>Export as GLB…</button>
            </>
          )}
          {['nkm', 'kcl'].includes(
            snapshot?.files.find((f) => f.path === snapshot.selected)?.format ?? '',
          ) && <button onClick={() => openWorkspace('Track')}>Open in Track workspace</button>}
          {snapshot?.files.find((f) => f.path === snapshot.selected)?.format === 'kcl' && (
            <button onClick={() => void exportSelection('obj')}>Export as OBJ…</button>
          )}
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

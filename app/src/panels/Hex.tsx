import { useEffect, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { binary } from '../backend';
import { useStore } from '../store';
export function Hex() {
  const snapshot = useStore((s) => s.snapshot);
  const file = snapshot?.files.find((f) => f.path === snapshot.selected);
  const ref = useRef<HTMLDivElement>(null);
  const [chunk, setChunk] = useState({ start: 0, data: new Uint8Array() });
  const [error, setError] = useState('');
  const virtual = useVirtualizer({
    count: Math.ceil((file?.size ?? 0) / 16),
    getScrollElement: () => ref.current,
    estimateSize: () => 22,
    overscan: 12,
  });
  const rows = virtual.getVirtualItems();
  const start = Math.floor(((rows[0]?.index ?? 0) * 16) / 4096) * 4096;
  const path = snapshot?.selected ?? '';
  const revision = snapshot?.history.filter((h) => h.applied).length ?? 0;
  useEffect(() => {
    if (!path) return;
    let active = true;
    void binary({
      path,
      mode: 'hex',
      start,
      length: 16384,
      palette: null,
      screen: null,
      texture: 0,
      bank: 0,
    })
      .then((b) => {
        if (active) {
          setChunk({ start, data: new Uint8Array(b) });
          setError('');
        }
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, [path, start, revision]);
  useEffect(() => {
    ref.current?.scrollTo({ top: 0 });
  }, [path]);
  return (
    <div className="panel hex">
      <div className="hex-header mono">
        OFFSET <span>00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F</span>
        <span>DECODED TEXT</span>
      </div>
      <div className="hex-scroll" ref={ref}>
        <div style={{ height: virtual.getTotalSize(), position: 'relative' }}>
          {rows.map((row) => {
            const offset = row.index * 16;
            const data =
              offset < chunk.start
                ? new Uint8Array()
                : chunk.data.slice(offset - chunk.start, offset - chunk.start + 16);
            return (
              <div
                className="hex-row mono"
                key={row.key}
                style={{ position: 'absolute', top: row.start, height: row.size }}
              >
                <span className="offset">{offset.toString(16).padStart(8, '0').toUpperCase()}</span>
                <span className="hex-bytes">
                  {Array.from(data).map((b, i) => (
                    <span key={i} className={b === 0 ? 'zero' : ''}>
                      {b.toString(16).padStart(2, '0').toUpperCase()}{' '}
                    </span>
                  ))}
                </span>
                <span className="ascii">
                  {Array.from(data)
                    .map((b) => (b >= 32 && b < 127 ? String.fromCharCode(b) : '·'))
                    .join('')}
                </span>
              </div>
            );
          })}
        </div>
        {!file && <div className="empty">Select a file to view its bytes.</div>}
        {error && <div className="inline-error">{error}</div>}
      </div>
      <div className="panel-foot">
        Read only <span>16 bytes / row · hexadecimal</span>
      </div>
    </div>
  );
}

import { ChevronDown, Info, ShieldQuestion } from 'lucide-react';
import { useStore } from '../store';
export const bytes = (n: number) =>
  n >= 1024 * 1024
    ? `${(n / 1024 / 1024).toFixed(2)} MiB`
    : n >= 1024
      ? `${(n / 1024).toFixed(1)} KiB`
      : `${n} B`;
export function Properties() {
  const snapshot = useStore((s) => s.snapshot);
  const file = snapshot?.files.find((f) => f.path === snapshot.selected);
  return (
    <div className="panel properties">
      <div className="section-label">
        <ChevronDown size={12} /> FILE
      </div>
      {file ? (
        <>
          <div className="property-heading">
            <Info size={17} />
            <strong>{file.name}</strong>
          </div>
          <dl>
            <dt>Format</dt>
            <dd>{file.format.toUpperCase()}</dd>
            <dt>Category</dt>
            <dd>{file.category}</dd>
            <dt>Size</dt>
            <dd className="mono">{bytes(file.size)}</dd>
            <dt title="Absolute ROM offset for top-level files; uncompressed container offset for archive members">
              Offset
            </dt>
            <dd className="mono">0x{file.offset.toString(16).padStart(8, '0').toUpperCase()}</dd>
            <dt>Compression</dt>
            <dd>{file.compressed ? 'LZ77 · type 0x10' : 'None'}</dd>
            <dt>Source</dt>
            <dd>{snapshot?.edited.includes(file.path) ? 'Project edit' : 'ROM · read only'}</dd>
          </dl>
          <label className="property-path">
            Path<code>{file.path}</code>
          </label>
          {file.parent && (
            <p className="muted inset">Offset is relative to the uncompressed archive.</p>
          )}
        </>
      ) : (
        <div className="empty">Select a file to inspect it.</div>
      )}
      <div className="section-label">
        <ChevronDown size={12} /> COMPATIBILITY
      </div>
      <div className="compat-card">
        <ShieldQuestion size={18} />
        <div>
          {snapshot?.compatibility ?? 'Unknown'}
          <p>
            Format and budget checks for the project. Test exported ROMs in your emulator or on
            hardware.
          </p>
        </div>
      </div>
      <div className="section-label">
        <ChevronDown size={12} /> VALIDATION
      </div>
      {snapshot?.issues.length ? (
        snapshot.issues.map((i, n) => (
          <div className="issue" key={n}>
            <strong>{i.message}</strong>
            <p>{i.why}</p>
          </div>
        ))
      ) : (
        <p className="inset muted">
          {snapshot?.manifest
            ? 'No issues reported by current format and budget checks.'
            : 'Create a project to validate your edits.'}
        </p>
      )}
    </div>
  );
}

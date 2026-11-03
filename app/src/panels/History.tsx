import { History as HistoryIcon, Undo2, Redo2, Check } from 'lucide-react';
import { useStore } from '../store';
export function History() {
  const { snapshot, run } = useStore();
  return (
    <div className="panel history">
      <div className="toolbar">
        <span>PROJECT HISTORY</span>
        <button
          title="Undo"
          aria-label="Undo"
          disabled={!snapshot?.can_undo}
          onClick={() => void run({ kind: 'Undo' })}
        >
          <Undo2 size={14} />
        </button>
        <button
          title="Redo"
          aria-label="Redo"
          disabled={!snapshot?.can_redo}
          onClick={() => void run({ kind: 'Redo' })}
        >
          <Redo2 size={14} />
        </button>
      </div>
      <div className="history-list">
        {snapshot?.history.map((h, i) => (
          <div className={`history-row ${h.applied ? '' : 'muted'}`} key={i}>
            <span className="history-index">{String(i + 1).padStart(2, '0')}</span>
            <Check size={12} style={{ visibility: h.applied ? 'visible' : 'hidden' }} />
            <span>{h.title}</span>
          </div>
        ))}
        {!snapshot?.history.length && (
          <div className="empty small">
            <HistoryIcon size={20} />
            <p>
              Your edits appear here.
              <br />
              Undo works across every workspace.
            </p>
          </div>
        )}
      </div>
      <div className="panel-foot">
        {snapshot?.history.filter((h) => h.applied).length ?? 0} applied{' '}
        <span>Ctrl Z · Ctrl Shift Z</span>
      </div>
    </div>
  );
}

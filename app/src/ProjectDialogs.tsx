import { useState } from 'react';
import { useStore } from './store';
import { pickDirectory, pickFile, savePath } from './backend';
import { bytes } from './panels/Properties';
export function ExportDialog() {
  const { snapshot, run, busy, set, error } = useStore();
  const [rom, setRom] = useState('');
  const [bps, setBps] = useState('');
  const result = snapshot?.ds_export;
  return (
    <div className="modal-backdrop">
      <section
        className="modal project-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Export for DS"
      >
        <div className="modal-header">
          <strong>Export for DS</strong>
          <div className="grow" />
          <button onClick={() => set({ exportOpen: false })}>Close</button>
        </div>
        <div className="dialog-content">
          {error && (
            <p className="issue" role="alert">
              {error}
            </p>
          )}
          <p>
            Build from your clean ROM, project base and current edits. Share the BPS with other
            players.
          </p>
          <label>
            Patched ROM (for my DS / flashcart / emulator)
            <div className="path-row">
              <input
                aria-label="Patched ROM output"
                value={rom}
                onChange={(e) => setRom(e.target.value)}
              />
              <button
                onClick={() =>
                  void savePath('patched.nds').then((p) => {
                    if (p) setRom(p);
                  })
                }
              >
                Choose ROM output…
              </button>
            </div>
          </label>
          <label>
            BPS patch (to share)
            <div className="path-row">
              <input aria-label="BPS output" value={bps} onChange={(e) => setBps(e.target.value)} />
              <button
                onClick={() =>
                  void savePath('hack.bps').then((p) => {
                    if (p) setBps(p);
                  })
                }
              >
                Choose patch output…
              </button>
            </div>
          </label>
          {snapshot?.base && (
            <p className="issue">
              Based on {snapshot.base.source_name}. Publishing requires the original author's credit
              or permission.
            </p>
          )}
          <button
            className="accent"
            disabled={busy || (!rom && !bps)}
            onClick={() => void run({ kind: 'ExportDs', rom: rom || null, bps: bps || null })}
          >
            Build DS output
          </button>
          {result && (
            <div className="export-result" role="status">
              <h3>Export complete</h3>
              <p>
                {result.changed.length} changed files · {result.in_place.length} in place ·{' '}
                {result.moved.length} moved
              </p>
              <p>
                ROM: {bytes(result.rom_size)}{' '}
                {result.patch_size !== null && `· BPS: ${bytes(result.patch_size)}`}
              </p>
              {result.changed.map((p) => (
                <div className="mono" key={p}>
                  {p}
                </div>
              ))}
              {result.warnings.map((w) => (
                <p className="issue" key={w}>
                  {w}
                </p>
              ))}
              {[result.rom_written, result.patch_written]
                .filter((p): p is string => !!p)
                .map((p) => (
                  <div key={p}>
                    <code>{p}</code>{' '}
                    <button onClick={() => void run({ kind: 'ShowInFolder', path: p })}>
                      Show in folder
                    </button>
                  </div>
                ))}
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
export function CreateDialog() {
  const { run, busy, set, error } = useStore();
  const [kind, setKind] = useState('clean');
  const [source, setSource] = useState('');
  const [path, setPath] = useState('');
  return (
    <div className="modal-backdrop">
      <section
        className="modal project-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="New project"
      >
        <div className="modal-header">
          <strong>New project from…</strong>
          <div className="grow" />
          <button onClick={() => set({ createOpen: false })}>Cancel</button>
        </div>
        <div className="dialog-content">
          {error && (
            <p className="issue" role="alert">
              {error}
            </p>
          )}
          <label>
            Base
            <select
              aria-label="Project base"
              value={kind}
              onChange={(e) => {
                setKind(e.target.value);
                setSource('');
              }}
            >
              <option value="clean">My clean ROM</option>
              <option value="nds">A modded ROM</option>
              <option value="bps">A BPS patch</option>
            </select>
          </label>
          {kind !== 'clean' && (
            <label>
              Source
              <div className="path-row">
                <input
                  aria-label="Base source"
                  value={source}
                  onChange={(e) => setSource(e.target.value)}
                />
                <button
                  onClick={() =>
                    void pickFile([kind]).then((p) => {
                      if (p) setSource(p);
                    })
                  }
                >
                  Choose base…
                </button>
              </div>
            </label>
          )}
          <label>
            Project folder
            <div className="path-row">
              <input
                aria-label="New project folder"
                value={path}
                onChange={(e) => setPath(e.target.value)}
              />
              <button
                onClick={() =>
                  void pickDirectory().then((p) => {
                    if (p) setPath(p);
                  })
                }
              >
                Choose folder…
              </button>
            </div>
          </label>
          <p>Imported bases are stored as base.bps with their source name and hash.</p>
          <button
            className="accent"
            disabled={busy || !path || (kind !== 'clean' && !source)}
            onClick={() => {
              const manifest = {
                id: 'author.my-mod',
                name: 'Untitled mod',
                author: 'Author',
                version: '0.1.0',
                type: 'texture',
                platforms: ['pc', 'ds'],
                description: '',
              };
              void run(
                kind === 'clean'
                  ? { kind: 'CreateProject', path, manifest }
                  : { kind: 'CreateFromBase', path, manifest, source },
              ).then(() => {
                if (!useStore.getState().error) set({ createOpen: false });
              });
            }}
          >
            Create project
          </button>
        </div>
      </section>
    </div>
  );
}

import { FileLink, LinkedText } from '../files';
import { Resizer } from '../Resizer';
import { useEffect, useState } from 'react';
import {
  ArrowUpRight,
  Package,
  FolderOpen,
  Palette,
  Type,
  RotateCcw,
  CheckCircle2,
  FileDiff,
  BookOpen,
  ChevronRight,
} from 'lucide-react';
import { useStore, draft as registerDraft } from '../store';
import { commands, invokeCommand, openWorkspace } from '../commands';
import { bytes } from './Properties';
import type { Manifest } from '../bindings';
export function Overview() {
  const { snapshot, run, busy, set } = useStore();
  const [draft, setDraft] = useState<Manifest | null>(null);
  const serializedManifest = JSON.stringify(snapshot?.manifest ?? null);
  useEffect(
    () => setDraft(JSON.parse(serializedManifest) as Manifest | null),
    [serializedManifest],
  );
  if (!snapshot?.manifest)
    return (
      <div className="panel onboarding">
        <div className="onboarding-inner">
          <div className="brand-symbol large">T</div>
          <span className="eyebrow">MKDSRUST TOOLBOX / FOUNDATION</span>
          <h1>A place for your next idea.</h1>
          <p>
            Explore your own ROM. Make it yours, one palette
            <br />
            or message at a time.
          </p>
          <div className="setup-step">
            <span>01</span>
            <div>
              <strong>Open your ROM</strong>
              <p>AMCE · USA revision 0. Your original stays read only.</p>
            </div>
            <button
              className="accent"
              onClick={() => invokeCommand(commands.find((c) => c.id === 'file.openRom')!)}
            >
              <FolderOpen size={14} />
              {snapshot?.rom_path ? 'Change ROM' : 'Choose ROM'}
            </button>
          </div>
          <div className="setup-step">
            <span>02</span>
            <div>
              <strong>Start a project</strong>
              <p>Edits and project information live in your own folder.</p>
            </div>
            <button
              disabled={!snapshot?.rom_path}
              onClick={() => invokeCommand(commands.find((c) => c.id === 'project.create')!)}
            >
              Create project
              <ChevronRight size={14} />
            </button>
          </div>
          <button
            className="text-button"
            onClick={() => invokeCommand(commands.find((c) => c.id === 'project.open')!)}
          >
            Open an existing project <ArrowUpRight size={14} />
          </button>
          <div className="local-note">
            <CheckCircle2 size={14} />
            Local files. Reversible edits. Your ROM stays yours.
          </div>
        </div>
      </div>
    );
  const changeDraft = (manifest: Manifest) => {
    setDraft(manifest);
    registerDraft(
      'manifest',
      JSON.stringify(manifest) === JSON.stringify(snapshot.manifest)
        ? null
        : { kind: 'UpdateManifest', manifest },
    );
  };
  const dirty = JSON.stringify(draft) !== JSON.stringify(snapshot.manifest);
  return (
    <div className="panel overview">
      <div className="overview-heading">
        <div>
          <div className="eyebrow">PROJECT / OVERVIEW</div>
          <h1>{snapshot.manifest.name}</h1>
          <p>{snapshot.manifest.description || 'Your next mod starts here.'}</p>
        </div>
        <span className="project-version mono">v{snapshot.manifest.version}</span>
      </div>
      <button className="accent ds-export-button" onClick={() => set({ exportOpen: true })}>
        Export for DS…
      </button>
      {snapshot.base && (
        <div className="overview-section">
          <h2>What this hack changes</h2>
          <p>
            Base: {snapshot.base.source_name} · <code>{snapshot.base.source_sha256}</code>
          </p>
          {snapshot.base_changes.map((c) => (
            <div className="change-row" key={c.path}>
              <FileLink path={c.path} />
              <span>{c.change}</span>
            </div>
          ))}
          <p className="issue">Publishing needs the original author's credit or permission.</p>
        </div>
      )}
      <div className="overview-stats">
        <div>
          <FileDiff size={19} />
          <strong>{snapshot.edited.length}</strong>
          <span>edited files</span>
        </div>
        <div>
          <FolderOpen size={19} />
          <strong>{snapshot.files.length}</strong>
          <span>explorable files</span>
        </div>
        <div>
          <CheckCircle2 size={19} />
          <strong>{snapshot.game}</strong>
          <span>target game code</span>
        </div>
      </div>
      <div className="overview-section">
        <div className="overview-section-title">
          <h2>Changes</h2>
          <span className="pill">{snapshot.edited.length} files</span>
          <div className="grow" />
          <button className="accent" disabled={busy} onClick={() => void run({ kind: 'Pack' })}>
            <Package size={14} />
            Pack mod
          </button>
        </div>
        <div className="changes-head">
          <span>FILE</span>
          <span>CHANGE</span>
          <span />
        </div>
        {snapshot.edited.map((path) => (
          <div className="change-row" key={path}>
            <FileLink path={path} />
            <span className="change-tag">Modified</span>
            <button
              aria-label={`Revert ${path}`}
              title="Revert file (undoable)"
              onClick={() => void run({ kind: 'Revert', path })}
            >
              <RotateCcw size={13} />
            </button>
          </div>
        ))}
        {!snapshot.edited.length && (
          <div className="empty small">
            Your project is ready. Edit a palette or a message to begin.
          </div>
        )}
        {snapshot.pack && (
          <div className="pack-result" role="status">
            <Package size={18} />
            <div>
              <strong>Package built · {bytes(snapshot.pack.size)}</strong>
              <code>{snapshot.pack.path}</code>
              {snapshot.pack.warnings.map((w) => (
                <p key={w}>
                  <LinkedText text={w} />
                </p>
              ))}
            </div>
          </div>
        )}
      </div>
      <div className="overview-columns">
        <section className="overview-section">
          <div className="overview-section-title">
            <h2>Project information</h2>
          </div>
          {draft && (
            <form
              className="manifest-form"
              onSubmit={(e) => {
                e.preventDefault();
                void run({ kind: 'UpdateManifest', manifest: draft }).then(() => {
                  if (!useStore.getState().error) registerDraft('manifest', null);
                });
              }}
            >
              {(['name', 'id', 'version', 'author'] as const).map((key) => (
                <label key={key}>
                  {key === 'id' ? 'Package ID' : key}
                  <input
                    aria-label={`Project ${key}`}
                    value={draft[key]}
                    onChange={(e) => changeDraft({ ...draft, [key]: e.target.value })}
                  />
                </label>
              ))}
              <label>
                Type
                <select
                  value={draft.type}
                  onChange={(e) => changeDraft({ ...draft, type: e.target.value })}
                >
                  {['character', 'track', 'kart', 'battle', 'music', 'texture', 'other'].map(
                    (t) => (
                      <option key={t}>{t}</option>
                    ),
                  )}
                </select>
              </label>
              <label>
                Description
                <textarea
                  aria-label="Project description"
                  maxLength={2000}
                  value={draft.description ?? ''}
                  onChange={(e) => changeDraft({ ...draft, description: e.target.value })}
                />
              </label>
              <label>
                Platforms
                <select
                  value={draft.platforms?.includes('ds') ? 'ds' : 'pc'}
                  onChange={(e) =>
                    changeDraft({
                      ...draft,
                      platforms: e.target.value === 'ds' ? ['pc', 'ds'] : ['pc'],
                    })
                  }
                >
                  <option value="pc">PC only</option>
                  <option value="ds">PC + DS (requires full checks)</option>
                </select>
              </label>
              <button type="submit" disabled={!dirty}>
                Save project information
              </button>
            </form>
          )}
        </section>
        <Resizer name="overview-left" min={280} max={1400} />
        <section className="overview-section start-editing">
          <div className="overview-section-title">
            <h2>Start editing</h2>
          </div>
          <button onClick={() => openWorkspace('Texture & 2D')}>
            <Palette size={20} />
            <div>
              <strong>Find your colours</strong>
              <p>Edit palettes and preview 2D assets.</p>
            </div>
            <ArrowUpRight size={16} />
          </button>
          <button onClick={() => openWorkspace('Text')}>
            <Type size={20} />
            <div>
              <strong>Make every word count</strong>
              <p>Edit messages, keep control codes intact.</p>
            </div>
            <ArrowUpRight size={16} />
          </button>
          <div className="reference-note">
            <BookOpen size={16} />
            <p>Use the Explorer to inspect files, nested archives and raw bytes.</p>
          </div>
          <div className="target-info">
            <span className="text-kicker">TARGET ROM / SHA-256</span>
            <code>{snapshot.rom_hash}</code>
            <p>Toolbox verifies this hash whenever you open the project.</p>
          </div>
        </section>
      </div>
    </div>
  );
}

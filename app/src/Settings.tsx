import { useState } from 'react';
import { X, FolderOpen, Keyboard, Settings as SettingsIcon } from 'lucide-react';
import { useStore } from './store';
import { commands } from './commands';
import { pickDirectory, pickRom } from './backend';
export function Settings() {
  const { snapshot, run, set } = useStore();
  const [draft, setDraft] = useState(snapshot?.settings);
  const [tab, setTab] = useState('General');
  if (!draft) return null;
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) set({ settingsOpen: false });
      }}
    >
      <section className="settings modal" role="dialog" aria-modal="true" aria-label="Settings">
        <div className="modal-header">
          <SettingsIcon size={16} />
          <strong>Settings</strong>
          <div className="grow" />
          <button aria-label="Close settings" onClick={() => set({ settingsOpen: false })}>
            <X size={16} />
          </button>
        </div>
        <div className="settings-body">
          <aside>
            {['General', 'Keybindings'].map((t) => (
              <button className={tab === t ? 'selected' : ''} key={t} onClick={() => setTab(t)}>
                {t === 'General' ? <SettingsIcon size={14} /> : <Keyboard size={14} />} {t}
              </button>
            ))}
          </aside>
          <main>
            {tab === 'General' ? (
              <>
                <h2>Make yourself at home.</h2>
                <p className="hint">Settings are saved for your user account.</p>
                <label className="settings-field">
                  ROM path
                  <div>
                    <input
                      value={draft.rom_path}
                      onChange={(e) => setDraft({ ...draft, rom_path: e.target.value })}
                    />
                    <button
                      aria-label="Choose ROM path"
                      onClick={() => {
                        void pickRom().then((path) => {
                          if (path) setDraft({ ...draft, rom_path: path });
                        });
                      }}
                    >
                      <FolderOpen size={14} />
                    </button>
                  </div>
                </label>
                <label className="settings-field">
                  MKDSrust portable install
                  <div>
                    <input
                      value={draft.install_path}
                      placeholder="Folder containing data/ and games/"
                      onChange={(e) => setDraft({ ...draft, install_path: e.target.value })}
                    />
                    <button
                      aria-label="Choose install folder"
                      onClick={() => {
                        void pickDirectory().then((path) => {
                          if (path) setDraft({ ...draft, install_path: path });
                        });
                      }}
                    >
                      <FolderOpen size={14} />
                    </button>
                  </div>
                </label>
                <label className="settings-field">
                  Theme
                  <select
                    aria-label="Theme"
                    value={draft.theme}
                    onChange={(e) => setDraft({ ...draft, theme: e.target.value })}
                  >
                    <option value="dark">Dark</option>
                    <option value="light">Light</option>
                  </select>
                </label>
                <label className="settings-field">
                  Viewport navigation
                  <select
                    value={draft.navigation}
                    onChange={(e) => setDraft({ ...draft, navigation: e.target.value })}
                  >
                    {['Blender', 'Maya', 'MKTB'].map((n) => (
                      <option key={n}>{n}</option>
                    ))}
                  </select>
                  <small>Saved for the 3D workspaces in later stages.</small>
                </label>
              </>
            ) : (
              <>
                <h2>Keybindings</h2>
                <p className="hint">
                  Use Ctrl, Shift or Alt with a key. Ctrl also matches Command on macOS.
                </p>
                <div className="keybinding-head">
                  <span>COMMAND</span>
                  <span>SHORTCUT</span>
                </div>
                {commands.map((c) => (
                  <label className="keybinding-row" key={c.id}>
                    <span>{c.title}</span>
                    <input
                      aria-label={`Shortcut for ${c.title}`}
                      placeholder={c.shortcut ?? 'Unassigned'}
                      value={draft.keybindings[c.id] ?? c.shortcut ?? ''}
                      onChange={(e) =>
                        setDraft({
                          ...draft,
                          keybindings: { ...draft.keybindings, [c.id]: e.target.value },
                        })
                      }
                    />
                  </label>
                ))}
              </>
            )}
          </main>
        </div>
        <div className="modal-footer">
          <span>Preferences apply to every project.</span>
          <button onClick={() => set({ settingsOpen: false })}>Cancel</button>
          <button
            className="accent"
            onClick={() => {
              void run({ kind: 'Settings', settings: draft }).then(() => {
                if (!useStore.getState().error) set({ settingsOpen: false });
              });
            }}
          >
            Save settings
          </button>
        </div>
      </section>
    </div>
  );
}

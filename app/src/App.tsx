import { ExportDialog, CreateDialog } from './ProjectDialogs';
import { useEffect, useRef, useState } from 'react';
import {
  DockviewReact,
  type DockviewReadyEvent,
  type DockviewApi,
  type IDockviewPanelProps,
} from 'dockview';
import {
  PanelLeft,
  Play,
  ShieldQuestion,
  Search,
  ChevronDown,
  Settings as SettingsIcon,
  Check,
  LoaderCircle,
  X,
  Package,
  LayoutDashboard,
  Files,
  Palette as PaletteIcon,
  Type,
  HelpCircle,
  LockKeyhole,
} from 'lucide-react';
import { useStore, workspaces, stages } from './store';
import { commands, invokeCommand, matchesKey, openWorkspace } from './commands';
import { isMock } from './backend';
import { Overview } from './panels/Overview';
import { Explorer } from './panels/Explorer';
import { History } from './panels/History';
import { Hex } from './panels/Hex';
import { Properties } from './panels/Properties';
import { Texture, Palette } from './panels/Texture';
import { MessageList, TextEditor } from './panels/Text';
import { Settings } from './Settings';
import { CommandPalette } from './CommandPalette';
import { SceneViewport, SceneOutliner, SceneProperties, Overlays, Timeline } from './panels/Scene';
import { TestBench, GameLog } from './panels/TestBench';
import { useScene } from './scene-view';
import { AudioWorkspace } from './panels/Audio';
import { Wizards } from './Wizards';
const components: Record<string, React.FunctionComponent<IDockviewPanelProps>> = {
  overview: Overview,
  audio: AudioWorkspace,
  explorer: Explorer,
  history: History,
  hex: Hex,
  properties: Properties,
  texture: Texture,
  palette: Palette,
  messages: MessageList,
  text: TextEditor,
  preview: Preview,
  placeholder: Placeholder,
  scene: SceneViewport,
  sceneOutliner: SceneOutliner,
  sceneProperties: SceneProperties,
  overlays: Overlays,
  timeline: Timeline,
  bench: TestBench,
  logs: GameLog,
};
function Preview() {
  const snapshot = useStore((s) => s.snapshot);
  const file = snapshot?.files.find((f) => f.path === snapshot.selected);
  if (file?.format === 'bmg') return <TextEditor />;
  if (file && ['nclr', 'ncgr', 'nscr', 'nsbtx', 'nsbmd', 'ncer', 'nanr'].includes(file.format))
    return <Texture />;
  return (
    <div className="panel empty">
      <Files size={30} />
      <p>{file ? 'No preview for this format.' : 'Select a file in the Explorer.'}</p>
    </div>
  );
}
function Placeholder() {
  const workspace = useStore((s) => s.workspace);
  return (
    <div className="panel empty placeholder">
      <LockKeyhole size={32} />
      <span className="eyebrow">WORKSPACE / {workspace.toUpperCase()}</span>
      <h1>Room for what comes next.</h1>
      <p>
        {workspace} is coming in stage {stages[workspace]}.
      </p>
    </div>
  );
}
function Dock() {
  const workspace = useStore((s) => s.workspace);
  const api = useRef<DockviewApi>();
  function ready(event: DockviewReadyEvent) {
    const dock = event.api;
    api.current = dock;
    const saved = localStorage.getItem(`layout:${workspace}`);
    let restored = false;
    if (saved) {
      try {
        dock.fromJSON(JSON.parse(saved));
        restored = true;
      } catch {
        localStorage.removeItem(`layout:${workspace}`);
      }
    }
    if (!restored) {
      const center = dock.addPanel({
        id: 'main',
        component:
          workspace === 'Overview'
            ? 'overview'
            : workspace === 'Explorer'
              ? 'preview'
              : workspace === 'Texture & 2D'
                ? 'texture'
                : workspace === 'Text'
                  ? 'text'
                  : workspace === 'Model' || workspace === 'Track' || workspace === 'Rig & Animate'
                    ? 'scene'
                    : workspace === 'Audio'
                      ? 'audio'
                      : workspace === 'Test Bench'
                        ? 'bench'
                        : 'placeholder',
        title:
          workspace === 'Overview'
            ? 'Project dashboard'
            : workspace === 'Explorer'
              ? 'Preview'
              : workspace === 'Texture & 2D'
                ? 'Image editor'
                : workspace === 'Text'
                  ? 'Message editor'
                  : workspace,
      });
      const left = dock.addPanel({
        id: 'outliner',
        component:
          workspace === 'Text'
            ? 'messages'
            : workspace === 'Track' || workspace === 'Model' || workspace === 'Rig & Animate'
              ? 'sceneOutliner'
              : 'explorer',
        title:
          workspace === 'Text'
            ? 'Messages'
            : workspace === 'Track' || workspace === 'Model' || workspace === 'Rig & Animate'
              ? 'Outliner'
              : 'Explorer',
        position: { referencePanel: center.id, direction: 'left' },
        initialWidth: workspace === 'Text' ? 290 : 270,
      });
      const right = dock.addPanel({
        id: 'properties',
        component:
          workspace === 'Texture & 2D'
            ? 'palette'
            : workspace === 'Model' || workspace === 'Track' || workspace === 'Rig & Animate'
              ? 'sceneProperties'
              : 'properties',
        title: workspace === 'Texture & 2D' ? 'Palette' : 'Properties',
        position: { referencePanel: center.id, direction: 'right' },
        initialWidth: workspace === 'Texture & 2D' ? 300 : 260,
      });
      dock.addPanel({
        id: 'history',
        component: 'history',
        title: 'History',
        position: { referencePanel: left.id, direction: 'below' },
        initialHeight: 200,
      });
      if (workspace === 'Explorer') {
        dock.addPanel({
          id: 'hex',
          component: 'hex',
          title: 'Hex view',
          position: { referencePanel: center.id, direction: 'below' },
          initialHeight: 235,
        });
      }
      if (workspace === 'Model' || workspace === 'Rig & Animate')
        dock.addPanel({
          id: 'timeline',
          component: 'timeline',
          title: 'Animation timeline',
          position: { referencePanel: center.id, direction: 'below' },
          initialHeight: 130,
        });
      if (workspace === 'Track')
        dock.addPanel({
          id: 'overlays',
          component: 'overlays',
          title: 'Overlays',
          position: { referencePanel: right.id, direction: 'below' },
          initialHeight: 380,
        });
      if (workspace === 'Test Bench')
        dock.addPanel({
          id: 'logs',
          component: 'logs',
          title: 'Game output',
          position: { referencePanel: center.id, direction: 'below' },
          initialHeight: 280,
        });
      if (workspace === 'Overview') {
        dock.addPanel({
          id: 'properties-extra',
          component: 'properties',
          title: 'Selection',
          position: { referencePanel: right.id, direction: 'within' },
        });
      }
    }
    const saveLayout = () => {
      try {
        localStorage.setItem(`layout:${workspace}`, JSON.stringify(dock.toJSON()));
      } catch {
        /* Storage may be disabled in private sessions. */
      }
    };
    dock.onDidLayoutChange(saveLayout);
    saveLayout();
  }
  return <DockviewReact className="dockview-theme-abyss" components={components} onReady={ready} />;
}
export function App() {
  const {
    snapshot,
    workspace,
    settingsOpen,
    exportOpen,
    createOpen,
    paletteOpen,
    layoutVersion,
    error,
    notice,
    busy,
    set,
    run,
  } = useStore();
  const [menu, setMenu] = useState('');
  const [tour, setTour] = useState(false);
  const [compat, setCompat] = useState(false);
  const sceneInfo = useScene((s) => s.info);
  useEffect(() => {
    void run({ kind: 'Snapshot' });
  }, [run]);
  useEffect(() => {
    document.documentElement.dataset.theme = snapshot?.settings.theme ?? 'dark';
  }, [snapshot?.settings.theme]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        set({ settingsOpen: false, paletteOpen: false, exportOpen: false, createOpen: false });
        setMenu('');
        setTour(false);
        setCompat(false);
        return;
      }
      if (e.key === 'F3') {
        e.preventDefault();
        set({ paletteOpen: true });
        return;
      }
      const target = e.target as HTMLElement;
      const editing =
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable ||
        (target instanceof HTMLInputElement &&
          !['range', 'color', 'checkbox', 'button'].includes(target.type));
      const command = commands.find((c) => {
        const binding = snapshot?.settings.keybindings[c.id] ?? c.shortcut;
        return binding && matchesKey(e, binding);
      });
      if (command && (!editing || ['view.palette', 'view.settings'].includes(command.id))) {
        e.preventDefault();
        invokeCommand(command);
      }
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [snapshot, set]);
  const icons = [LayoutDashboard, Files, PaletteIcon, Type];
  return (
    <div
      className="app"
      onClick={() => {
        if (menu) setMenu('');
      }}
    >
      <header className="menubar">
        <div className="brand-symbol">T</div>
        <span className="app-name">
          MKDSrust <strong>Toolbox</strong>
        </span>
        <div className="menubar-divider" />
        {['File', 'Edit', 'View', 'Project'].map((m) => (
          <div className="menu-anchor" key={m}>
            <button
              className={menu === m ? 'active' : ''}
              onClick={(e) => {
                e.stopPropagation();
                setMenu(menu === m ? '' : m);
              }}
            >
              {m}
            </button>
            {menu === m && (
              <div className="menu" onClick={(e) => e.stopPropagation()}>
                {commands
                  .filter((c) => c.category === m)
                  .map((c) => (
                    <button
                      disabled={!c.when(snapshot)}
                      key={c.id}
                      onClick={() => {
                        invokeCommand(c);
                        setMenu('');
                      }}
                    >
                      <span>{c.title}</span>
                      <kbd>{snapshot?.settings.keybindings[c.id] ?? c.shortcut}</kbd>
                    </button>
                  ))}
              </div>
            )}
          </div>
        ))}
        <div className="grow" />
        <button disabled={!snapshot?.manifest} onClick={() => set({ wizardOpen: true })}>Beginner recipes</button>
        {isMock && <span className="demo-label">SYNTHETIC DEMO</span>}
        <span className="project-name">
          {snapshot?.manifest?.name ?? 'No project'}
          <ChevronDown size={12} />
        </span>
        <button
          className="icon-button"
          aria-label="Settings"
          title="Settings"
          onClick={() => set({ settingsOpen: true })}
        >
          <SettingsIcon size={15} />
        </button>
      </header>
      <nav className="workspace-bar" aria-label="Workspaces">
        <div className="workspace-tabs">
          {workspaces.map((w, i) => {
            const Icon = icons[i];
            return (
              <button
                key={w}
                title={stages[w] ? `Coming in stage ${stages[w]}` : w}
                disabled={!!stages[w]}
                className={workspace === w ? 'active' : ''}
                onClick={() => openWorkspace(w)}
              >
                {Icon && <Icon size={13} />}
                <span>{w}</span>
              </button>
            );
          })}
        </div>
        <button
          className="workspace-help icon-button"
          title={`${workspace} workspace tour`}
          aria-label="Workspace tour"
          onClick={() => setTour(!tour)}
        >
          <HelpCircle size={15} />
        </button>
        <button
          className="play"
          title="Pack, install and launch in MKDSrust · F5"
          disabled={!snapshot?.manifest || busy || snapshot?.bench.running}
          onClick={() => {
            openWorkspace('Test Bench');
            void run({ kind: 'Play', game: useScene.getState().game });
          }}
        >
          <Play size={12} />
          Play
        </button>
        {snapshot?.bench.running && (
          <button onClick={() => void run({ kind: 'Stop' })}>Stop</button>
        )}
        <div className="compat-anchor">
          <button className="compat-badge" onClick={() => setCompat(!compat)}>
            <ShieldQuestion size={14} />
            <span className="muted">
              {snapshot?.compatibility ?? 'Unknown'}
              {snapshot?.compatibility === 'PC only' ? ` (${snapshot.issues.length})` : ''}
            </span>
          </button>
          {compat && (
            <div className="popover">
              <strong>DS compatibility · {snapshot?.compatibility ?? 'Unknown'}</strong>
              <p>
                Checks cover the current project's formats and DS budgets. Hardware testing remains
                a separate step.
              </p>
              {snapshot?.issues.map((issue, i) => (
                <div className="issue" key={i}>
                  <button
                    onClick={() => {
                      openWorkspace('Explorer');
                      void run({ kind: 'Select', path: issue.location });
                    }}
                  >
                    {issue.location}
                  </button>
                  <p>{issue.message}</p>
                  <small>{issue.why}</small>
                </div>
              ))}
              <button disabled={!snapshot?.manifest} onClick={() => void run({ kind: 'Validate' })}>
                Recheck compatibility
              </button>
            </div>
          )}
        </div>
      </nav>
      <div className="breadcrumb">
        <PanelLeft size={13} />
        <span>{snapshot?.manifest?.name ?? 'Welcome'}</span>
        <span>/</span>
        <strong>{workspace}</strong>
        <div className="grow" />
        {(workspace === 'Model' || workspace === 'Track' || workspace === 'Rig & Animate') && sceneInfo && (
          <span>
            {sceneInfo.budget.polygons.toLocaleString()} polys ·{' '}
            {sceneInfo.budget.vertices.toLocaleString()} verts · {sceneInfo.issues.length} course
            issues
          </span>
        )}
        <button onClick={() => set({ paletteOpen: true })}>
          <Search size={12} />
          Search commands<kbd>Ctrl Shift P</kbd>
        </button>
      </div>
      {tour && (
        <div className="tour">
          <HelpCircle size={17} />
          <div>
            <strong>{workspace}</strong>
            <p>
              {workspace === 'Overview'
                ? 'Edit your project information, review changes and build a mod package.'
                : workspace === 'Explorer'
                  ? 'Browse nested archives. Right click a file to extract or export it. Double click palettes and messages to edit.'
                  : workspace === 'Text'
                    ? 'Pick a language file, select a message, then edit text around the control chips. Save with Ctrl Enter.'
                    : 'Pair tiles with a palette and optional screen. Drag to pan, zoom in for pixels, and Shift click swatches to adjust a range.'}{' '}
              Drag panel tabs to rearrange the layout. View → Reset restores the default.
            </p>
          </div>
          <button aria-label="Close tour" onClick={() => setTour(false)}>
            <X size={14} />
          </button>
        </div>
      )}
      {error && (
        <div className="error-banner" role="alert">
          <strong>Action failed</strong>
          <span>{error}</span>
          <button aria-label="Dismiss error" onClick={() => set({ error: '' })}>
            <X size={14} />
          </button>
        </div>
      )}
      {snapshot?.warnings.map((w) => (
        <div className="warning-banner" key={w}>
          {w}
        </div>
      ))}
      <main className="workspace" data-testid="workspace">
        <Dock key={`${workspace}:${layoutVersion}`} />
      </main>
      <footer className="statusbar">
        <span className="status-icon">
          {busy ? <LoaderCircle size={13} className="spin" /> : <Check size={13} />}
        </span>
        <span>{busy ? 'Working…' : notice}</span>
        <span className="status-separator" />
        <span>{snapshot?.selected || 'Select a file to get started'}</span>
        <div className="grow" />
        <button disabled={!snapshot?.manifest} onClick={() => void run({ kind: 'Validate' })}>
          {snapshot?.issues.length ?? 0} issues
        </button>
        <span className="status-separator" />
        <span>
          <Package size={12} />
          {snapshot?.edited.length ?? 0} edits
        </span>
        <span className="stage-label">DS AUTHORING</span>
      </footer>
      {settingsOpen && <Settings />}
      {useStore((s) => s.wizardOpen) && <Wizards />}
      {exportOpen && <ExportDialog />}
      {createOpen && <CreateDialog />}
      {paletteOpen && <CommandPalette />}
    </div>
  );
}

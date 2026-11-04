import { useStore, storedList, type Recipe } from '../store';
import { commands, invokeCommand, openWorkspace } from '../commands';
const cards: [string, string, Recipe | 'Images' | 'Text'][] = [
  [
    'Replace a character',
    'Bring a model, fit it to a driver and inherit animations and voice.',
    'Character',
  ],
  ['Replace a kart', 'Fit a body, choose wheels and inherit driving stats.', 'Kart'],
  [
    'Make a track',
    'Import a map, draw a racing line and generate collision and a minimap.',
    'Track',
  ],
  ['Change music', 'Listen to music and sounds, then replace them with your own audio.', 'Music'],
  [
    'Recolour or redraw a menu / HUD / item',
    'Choose an image, swap a colour, paint or import a PNG.',
    'Images',
  ],
  ['Change text and names', 'Edit English (US) menus, course names and messages.', 'Text'],
  ['Make a cup', 'Choose four courses and name your Grand Prix.', 'Cups'],
];
export function Home() {
  const { snapshot, set, run } = useStore();
  const recent = storedList('recent:' + snapshot?.rom_hash);
  return (
    <div className="home panel">
      <div className="home-heading">
        <div>
          <span className="eyebrow">YOUR GAME, YOUR IDEAS</span>
          <h1>What do you want to make?</h1>
          <p>Choose a task. Save your project, then export for your DS or emulator.</p>
        </div>
        <button onClick={() => openWorkspace('Game browser')}>Browse the game</button>
        <button disabled={!snapshot?.manifest} onClick={() => set({ exportOpen: true })}>
          Export for DS…
        </button>
      </div>
      {!snapshot?.rom_path && (
        <div className="welcome-actions">
          <p>Start with your own USA ROM. The original stays in place.</p>
          <button
            className="accent"
            onClick={() => invokeCommand(commands.find((c) => c.id === 'file.openRom')!)}
          >
            Open ROM…
          </button>
        </div>
      )}
      {snapshot?.rom_path && !snapshot.manifest && (
        <p>
          Create a project to keep your changes.{' '}
          <button onClick={() => set({ createOpen: true })}>Create project…</button>
        </p>
      )}
      <div className="home-cards">
        {cards.map(([title, description, flow]) => (
          <button
            className="home-card"
            key={title}
            disabled={!snapshot?.rom_path}
            onClick={() => {
              if (flow === 'Images') openWorkspace('Image gallery');
              else if (flow === 'Text') openWorkspace('Text');
              else if (flow === 'Music') openWorkspace('Audio');
              else set({ recipe: flow, wizardOpen: true });
            }}
          >
            <strong>{title}</strong>
            <span>{description}</span>
            <span className="card-link">Get started →</span>
          </button>
        ))}
      </div>
      <section>
        <h2>Recently opened</h2>
        <div className="recent-strip">
          {recent
            .filter((p) => snapshot?.files.some((f) => f.path === p))
            .map((path) => (
              <button
                key={path}
                onClick={() => {
                  void run({ kind: 'Select', path });
                  openWorkspace('Game browser');
                }}
              >
                {snapshot?.game_entries.find((e) => e.path === path)?.label ??
                  path.split('/').pop()}
              </button>
            ))}
          {!recent.length && <p>Your opened items will appear here.</p>}
        </div>
      </section>
      <section>
        <h2>Open Recent projects</h2>
        {storedList('recent-projects').map((path) => (
          <button
            className="recent-project"
            key={path}
            onClick={() => void run({ kind: 'OpenProject', path })}
          >
            {path}
          </button>
        ))}
        {!storedList('recent-projects').length && <p>No recent projects yet.</p>}
      </section>
    </div>
  );
}

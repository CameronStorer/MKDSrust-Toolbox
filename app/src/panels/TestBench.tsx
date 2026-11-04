import { useEffect } from 'react';
import { useStore } from '../store';
import { useScene } from '../scene-view';
import { bytes } from './Properties';
export function TestBench() {
  const { snapshot, run, busy, set } = useStore();
  const { game, set: setScene } = useScene();
  const bench = snapshot?.bench;
  const race = snapshot?.settings.race;
  const update = (field: keyof NonNullable<typeof race>, value: string) => {
    if (snapshot && race)
      void run({
        kind: 'Settings',
        settings: {
          ...snapshot.settings,
          race: { ...race, [field]: value === '' ? null : Number(value) },
        },
      });
  };
  useEffect(() => {
    if (bench?.games.length === 1 && game !== bench.games[0].id)
      setScene({ game: bench.games[0].id });
  }, [bench?.games, game, setScene]);
  useEffect(() => {
    if (!bench?.running) return;
    const timer = setInterval(() => {
      if (!useStore.getState().busy) void run({ kind: 'Snapshot' });
    }, 500);
    return () => clearInterval(timer);
  }, [bench?.running, run]);
  return (
    <div className="panel bench-panel">
      <div className="section-label">TEST BENCH / PLAY TARGETS</div>
      <div className="bench-content">
        <h2>Build. Test. Race.</h2>
        <p className="muted">
          Play your current ROM hack in a DS emulator, MKDSrust or on your flashcart.
        </p>
        <label>
          Play target
          <select
            aria-label="Play target"
            value={snapshot?.settings.play_target ?? 'emulator'}
            onChange={(e) => {
              if (snapshot)
                void run({
                  kind: 'Settings',
                  settings: { ...snapshot.settings, play_target: e.target.value },
                });
            }}
          >
            <option value="emulator">DS emulator</option>
            <option value="mkdsrust">MKDSrust</option>
            <option value="flashcart">Flashcart export folder</option>
          </select>
        </label>
        <p className="muted">
          Emulator: {snapshot?.settings.emulator_path || 'Set in Settings'} · Flashcart:{' '}
          {snapshot?.settings.flashcart_path || 'Set in Settings'}
        </p>
        <label>
          Portable install
          <code>{snapshot?.settings.install_path || 'Choose an install in Settings'}</code>
        </label>
        <button onClick={() => set({ settingsOpen: true })}>Configure install…</button>
        <label>
          Installed game
          <select
            aria-label="Installed game"
            value={game}
            onChange={(e) => setScene({ game: e.target.value })}
          >
            <option value="">Choose a native build</option>
            {bench?.games.map((g) => (
              <option value={g.id} key={g.id}>
                {g.name} · {g.id}
              </option>
            ))}
          </select>
        </label>
        <div className="bench-actions">
          <button
            className="accent"
            disabled={
              busy ||
              !snapshot?.manifest ||
              bench?.running ||
              (snapshot?.settings.play_target === 'mkdsrust' && !game)
            }
            onClick={() => void run({ kind: 'Play', game })}
          >
            Play target
          </button>
          <button disabled={!bench?.running || busy} onClick={() => void run({ kind: 'Stop' })}>
            Stop game
          </button>
          <span className={bench?.running ? 'running' : 'muted'}>
            {bench?.running ? '● Running' : '○ Stopped'}
          </span>
        </div>
        <div className="section-label">RACE SETUP · MKDSRUST</div>
        <button disabled={!game || busy} onClick={() => void run({ kind: 'Courses', game })}>
          Refresh courses
        </button>
        <label>
          Track
          <select
            aria-label="Race course"
            value={race?.course ?? ''}
            onChange={(e) => update('course', e.target.value)}
          >
            <option value="">Game default</option>
            {bench?.courses.map((c) => (
              <option key={c.id} value={c.id}>
                {c.id} · {c.name}
              </option>
            ))}
          </select>
        </label>
        {(['character', 'kart', 'racers'] as const).map((field) => (
          <label key={field}>
            {field}
            <input
              aria-label={field}
              type="number"
              min={field === 'racers' ? 1 : 0}
              max={field === 'racers' ? 8 : 255}
              placeholder="Game default"
              value={race?.[field] ?? ''}
              onChange={(e) => update(field, e.target.value)}
            />
          </label>
        ))}
        <label>
          Engine class
          <select disabled>
            <option>Game default (not supported yet)</option>
          </select>
        </label>
        <button
          disabled={
            busy || !snapshot?.manifest || (snapshot?.settings.play_target === 'mkdsrust' && !game)
          }
          onClick={() => void run({ kind: 'Reload', game })}
        >
          Reload · Ctrl+R
        </button>
        <p className="muted">Reload restarts the game with the same race and current edits.</p>
        <div className="section-label">LAST RUN</div>
        {bench?.last_run ? (
          <dl>
            <dt>Package</dt>
            <dd>{bytes(bench.last_run.package_size)}</dd>
            <dt>Installed mod</dt>
            <dd>
              <code>{bench.last_run.install_path}</code>
            </dd>
            <dt>Executable</dt>
            <dd>
              <code>{bench.last_run.executable}</code>
            </dd>
            <dt>Exit code</dt>
            <dd>
              {bench.last_run.exit_code ?? (bench.running ? 'Running' : 'Not started')}
              {bench.last_run.stopped ? ' · stopped by Toolbox' : ''}
            </dd>
          </dl>
        ) : (
          <p className="muted">No game launched in this session.</p>
        )}
      </div>
    </div>
  );
}
export function GameLog() {
  const logs = useStore((s) => s.snapshot?.bench.logs);
  return (
    <div className="panel game-log" role="log" aria-label="Game log">
      <pre>
        {logs?.length
          ? logs.join('')
          : 'Game stdout and stderr appear here.\nLatest output is retained up to 1 MiB.'}
      </pre>
    </div>
  );
}

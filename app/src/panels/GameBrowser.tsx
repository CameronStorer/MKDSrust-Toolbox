import { useEffect, useState } from 'react';
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import type { Entry } from '../bindings';
import { binary } from '../backend';
import { useStore, storedList } from '../store';
import { openFile, openWorkspace } from '../commands';
import { useView } from '../view';
import { Texture, Palette } from './Texture';
import { SceneViewport, Timeline } from './Scene';
import { TextEditor } from './Text';
import { AudioWorkspace } from './Audio';
const groups = ['Characters', 'Karts', 'Tracks', 'Menus & HUD', 'Items', 'Music & sounds', 'Text'];
export function chooseEntry(entry: Entry) {
  useView.getState().set({
    tiles: '',
    palette: entry.palette ?? '',
    screen: entry.screen ?? '',
    texture: 0,
    bank: 0,
  });
  void useStore.getState().run({ kind: 'Select', path: entry.path });
}
let thumbnailQueue = Promise.resolve();
export function Thumbnail({ entry }: { entry: Entry }) {
  const snapshot = useStore((s) => s.snapshot);
  const format = snapshot?.files.find((f) => f.path === entry.path)?.format;
  const [url, setUrl] = useState('');
  const revision = snapshot?.history.map((h) => (h.applied ? '1' : '0')).join('') ?? '';
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let alive = true;
    let object = '';
    let renderer: THREE.WebGLRenderer | undefined;
    let scene: THREE.Group | undefined;
    const model = format === 'nsbmd';
    if (!model && !['ncgr', 'ncer', 'nsbtx', 'nsbmd'].includes(format ?? '')) return;
    setFailed(false);
    thumbnailQueue = thumbnailQueue
      .then(async () => {
        if (!alive) return;
        const bytes = await binary({
          path: entry.path,
          mode: model ? 'glb' : format === 'ncer' ? 'cell-png' : 'png',
          palette: entry.palette,
          screen: entry.screen,
          texture: 0,
          bank: 0,
          start: 0,
          length: 0,
        });
        if (!alive) return;
        if (!model) {
          object = URL.createObjectURL(new Blob([bytes], { type: 'image/png' }));
          setUrl(object);
          return;
        }
        const gltf = await new GLTFLoader().parseAsync(bytes, '');
        scene = gltf.scene;
        if (!alive) {
          dispose(scene);
          return;
        }
        renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true });
        renderer.setSize(160, 100);
        renderer.setPixelRatio(1);
        const box = new THREE.Box3().setFromObject(scene);
        const center = box.getCenter(new THREE.Vector3());
        const size = box.getSize(new THREE.Vector3()).length() || 1;
        const camera = new THREE.PerspectiveCamera(40, 1.6, size / 1000, size * 20);
        const track = entry.group === 'Tracks';
        camera.position
          .copy(center)
          .add(
            new THREE.Vector3(
              track ? 0 : size * 0.7,
              track ? size * 1.8 : size * 0.6,
              track ? 0.001 : size * 1.4,
            ),
          );
        camera.lookAt(center);
        const world = new THREE.Scene();
        world.add(scene, new THREE.HemisphereLight(0xffffff, 0x778899, 3));
        renderer.render(world, camera);
        setUrl(renderer.domElement.toDataURL('image/png'));
        renderer.dispose();
        renderer.forceContextLoss();
        renderer = undefined;
        dispose(scene);
        scene = undefined;
      })
      .catch(() => {
        if (alive) setFailed(true);
      });
    return () => {
      alive = false;
      if (object) URL.revokeObjectURL(object);
      renderer?.dispose();
      renderer?.domElement.remove();
      if (scene) dispose(scene);
    };
  }, [entry.path, entry.palette, entry.screen, entry.group, format, revision]);
  return (
    <div className="thumbnail">
      {url ? (
        <img src={url} alt={entry.label} />
      ) : !['nsbmd', 'ncgr', 'ncer', 'nsbtx'].includes(format ?? '') || failed ? (
        <span>{failed ? 'Preview unavailable' : entry.group === 'Text' ? 'Aa' : '♪'}</span>
      ) : null}
    </div>
  );
}
function dispose(root: THREE.Object3D) {
  root.traverse((o) => {
    if (o instanceof THREE.Mesh) {
      o.geometry.dispose();
      for (const m of Array.isArray(o.material) ? o.material : [o.material]) {
        for (const value of Object.values(m)) if (value instanceof THREE.Texture) value.dispose();
        m.dispose();
      }
    }
  });
}
export function GameBrowser({ gallery = false }: { gallery?: boolean }) {
  const { snapshot, opened, navigation, navigationIndex, set, run } = useStore();
  const [query, setQuery] = useState('');
  const [group, setGroup] = useState('');
  const [limit, setLimit] = useState(40);
  const [recent, setRecent] = useState(false);
  const entries = snapshot?.game_entries ?? [];
  const images = entries.filter(
    (e) =>
      ['ncgr', 'ncer', 'nsbtx', 'nsbmd'].includes(
        snapshot?.files.find((f) => f.path === e.path)?.format ?? '',
      ) &&
      (e.palette ||
        snapshot?.files.find((f) => f.path === e.path)?.format === 'nsbtx' ||
        (e.group === 'Items' && e.path.endsWith('.nsbmd'))),
  );
  const list = (gallery ? images : entries).filter(
    (e) =>
      (!group || e.group === group) &&
      (!recent || storedList('recent:' + snapshot?.rom_hash).includes(e.path)) &&
      `${e.label} ${e.location} ${e.section} ${e.path}`.toLowerCase().includes(query.toLowerCase()),
  );
  const selected = entries.find((e) => e.path === snapshot?.selected);
  const format = snapshot?.files.find((f) => f.path === snapshot.selected)?.format;
  const navigate = (delta: number) => {
    const index = navigationIndex + delta;
    const path = navigation[index];
    if (path) {
      set({ navigationIndex: index });
      const entry = entries.find((e) => e.path === path);
      if (entry) chooseEntry(entry);
      else void run({ kind: 'Select', path });
    }
  };
  return (
    <div className="game-browser panel">
      <aside className="game-tree">
        <h2>{gallery ? 'Image gallery' : 'Game browser'}</h2>
        <button className={!group ? 'active' : ''} onClick={() => setGroup('')}>
          All {gallery ? 'images' : 'game content'}
        </button>
        {groups.map((g) => (
          <details open key={g}>
            <summary onClick={() => setGroup(g)}>{g}</summary>
            {Array.from(
              new Set(
                (gallery ? images : entries).filter((e) => e.group === g).map((e) => e.section),
              ),
            ).map((section) => (
              <button
                key={section}
                onClick={() => {
                  setGroup(g);
                  setQuery(section);
                }}
              >
                {section}
              </button>
            ))}
          </details>
        ))}
        <button
          onClick={() => {
            setRecent(!recent);
            setGroup('');
            setQuery('');
          }}
          aria-pressed={recent}
        >
          Recently opened
        </button>
        <button onClick={() => openWorkspace('Explorer')}>Studio: raw files & pairing</button>
      </aside>
      <div className="game-content">
        <div className="toolbar">
          <button aria-label="Back" disabled={navigationIndex <= 0} onClick={() => navigate(-1)}>
            ←
          </button>
          <button
            aria-label="Forward"
            disabled={navigationIndex >= navigation.length - 1}
            onClick={() => navigate(1)}
          >
            →
          </button>
          <input
            aria-label="Search game"
            placeholder="Search names, where you see it, or file paths"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button
            onClick={() => {
              setQuery('');
              setGroup('');
            }}
          >
            Clear
          </button>
        </div>
        <div className="game-tabs" role="tablist" aria-label="Open items">
          {opened.map((path) => (
            <button
              key={path}
              role="tab"
              aria-selected={path === snapshot?.selected}
              onClick={() => {
                const e = entries.find((e) => e.path === path);
                if (e) chooseEntry(e);
              }}
            >
              {entries.find((e) => e.path === path)?.label ?? path.split('/').pop()}
            </button>
          ))}
        </div>
        <div className="game-results">
          {list.slice(0, limit).map((entry) => (
            <button
              className={`game-card ${snapshot?.selected === entry.path ? 'active' : ''}`}
              key={entry.path}
              onClick={() => chooseEntry(entry)}
            >
              <Thumbnail entry={entry} />
              <strong>{entry.label}</strong>
              <span>{entry.location}</span>
              <small>{entry.section}</small>
            </button>
          ))}
          {!list.length && (
            <p>
              No matching {gallery ? 'paired images' : 'items'}. Try a name or path, or inspect
              the Explorer in Studio.
            </p>
          )}
          {list.length > limit && (
            <button onClick={() => setLimit(limit + 40)}>
              Show more ({list.length - limit} remaining)
            </button>
          )}
        </div>
        {selected && (
          <section className="game-preview">
            <div className="toolbar">
              <strong>{selected.label}</strong>
              <span>{selected.location}</span>
              <div className="grow" />
              <button
                onClick={() =>
                  void openFile(
                    format === 'bmg'
                      ? 'Text'
                      : format === 'sdat'
                        ? 'Audio'
                        : format === 'nsbmd'
                          ? 'Model'
                          : 'Texture & 2D',
                    selected.path,
                  )
                }
              >
                Open editor
              </button>
            </div>
            <details>
              <summary>Details</summary>
              <code>{selected.path}</code>
            </details>
            {format === 'sdat' ? (
              <AudioWorkspace />
            ) : format === 'bmg' ? (
              <TextEditor />
            ) : ['nsbmd', 'kcl', 'nkm', 'nsbca'].includes(format ?? '') &&
              !(gallery && format === 'nsbmd') ? (
              <>
                <div className="browser-scene">
                  <SceneViewport />
                </div>
                {format === 'nsbca' && (
                  <div style={{ height: 130 }}>
                    <Timeline />
                  </div>
                )}
              </>
            ) : (
              <div className="gallery-editor">
                <Texture />
                <div>
                  <h3>Swap a colour</h3>
                  <p className="inset muted">
                    This palette is also used by{' '}
                    {Math.max(
                      0,
                      entries.filter((e) => !!selected.palette && e.palette === selected.palette)
                        .length - 1,
                    )}{' '}
                    other images.
                  </p>
                  <Palette />
                  <SharedImages palette={selected.palette} />
                </div>
              </div>
            )}
          </section>
        )}
      </div>
    </div>
  );
}
function SharedImages({ palette }: { palette: string | null }) {
  const snapshot = useStore((s) => s.snapshot);
  const shared = snapshot?.game_entries.filter((e) => !!palette && e.palette === palette) ?? [];
  return (
    <div className="shared-images">
      {shared
        .filter((e) => e.path !== snapshot?.selected)
        .map((e) => (
          <button key={e.path} onClick={() => chooseEntry(e)}>
            <Thumbnail entry={e} />
            {e.label}
          </button>
        ))}
    </div>
  );
}

import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { binary, inspectScene } from '../backend';
import { useStore } from '../store';
import { FileLink, openAsset } from '../files';
import type { RelatedAsset, RelatedPage } from '../bindings';
import { overlays, dispose } from './Scene';

export function showRelated(id: string) {
  useStore.getState().set({ relatedPage: id, workspace: 'Related assets' });
}
export function RelatedAssets() {
  const { snapshot, relatedPage, set } = useStore();
  const pages = snapshot?.related_pages ?? [];
  const current = pages.find((p) => p.id === relatedPage) ?? pages[0];
  const [query, setQuery] = useState('');
  return (
    <div className="related-browser">
      <aside className="related-nav">
        <h2>Related assets</h2>
        <input
          aria-label="Search related assets"
          placeholder="Find a character, kart or track"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        {['Character', 'Kart', 'Track'].map((kind) => (
          <details open key={kind}>
            <summary>{kind}s</summary>
            {pages
              .filter((p) => p.kind === kind && p.title.toLowerCase().includes(query.toLowerCase()))
              .map((p) => (
                <button
                  key={p.id}
                  className={current?.id === p.id ? 'active' : ''}
                  onClick={() => showRelated(p.id)}
                >
                  {p.title}
                </button>
              ))}
          </details>
        ))}
      </aside>
      {current ? (
        <section
          className="related-content"
          key={current.id}
          aria-label={`${current.title} related assets`}
        >
          <div className="toolbar">
            <h1>{current.title}</h1>
            <span>
              {current.kind} · slot {current.slot}
            </span>
            <div className="grow" />
            <button
              disabled={!snapshot?.manifest || (current.kind === 'Kart' && current.slot === 36)}
              onClick={() =>
                set({
                  recipe: current.kind as 'Character' | 'Kart' | 'Track',
                  recipeSlot: current.slot,
                  wizardOpen: true,
                })
              }
            >
              Replace {current.kind.toLowerCase()}…
            </button>
          </div>
          <dl className="related-stats">
            {current.stats.map(([name, value]) => (
              <div key={name}>
                <dt>{name}</dt>
                <dd>{name === 'Offsets' ? <FileLink path={value} /> : value}</dd>
              </div>
            ))}
          </dl>
          <div className="related-links">
            {current.links.map((id) => (
              <button key={id} onClick={() => showRelated(id)}>
                {pages.find((p) => p.id === id)?.title ?? id}
              </button>
            ))}
          </div>
          <div className="related-grid">
            {current.assets.map((asset, i) => (
              <AssetCard key={asset.label + i} asset={asset} page={current} />
            ))}
          </div>
        </section>
      ) : (
        <p className="empty">Open your ROM to see its characters, karts and tracks.</p>
      )}
    </div>
  );
}
function AssetCard({ asset, page }: { asset: RelatedAsset; page: RelatedPage }) {
  const set = useStore((s) => s.set);
  const host = useRef<HTMLElement>(null);
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => setVisible(entry.isIntersecting), {
      rootMargin: '100px',
    });
    if (host.current) observer.observe(host.current);
    return () => observer.disconnect();
  }, []);
  return (
    <article className="related-card" ref={host} aria-label={asset.label}>
      <h3>{asset.label}</h3>
      <p>{asset.usage}</p>
      {asset.path ? (
        <>
          <div className="related-preview">
            {visible &&
              (asset.mode === 'audio' ? (
                <AssetAudio asset={asset} />
              ) : ['image', 'cell'].includes(asset.mode) ? (
                <AssetImage asset={asset} />
              ) : (
                <AssetModel asset={asset} />
              ))}
          </div>
          <FileLink path={asset.path} />
          <button
            onClick={() => {
              if (asset.mode === 'audio' && page.kind === 'Track')
                set({ recipe: 'Music', recipeSlot: page.slot, wizardOpen: true });
              else if (asset.mode === 'cell')
                set({
                  recipe: 'Items',
                  recipeSlot: page.slot,
                  recipeAsset: asset.path!,
                  recipeCell: asset.index,
                  wizardOpen: true,
                });
              else if (asset.mode === 'image') {
                openAsset(asset.path!);
              } else
                set({
                  recipe: page.kind as 'Character' | 'Kart' | 'Track',
                  recipeSlot: page.slot,
                  wizardOpen: true,
                });
            }}
            disabled={page.kind === 'Kart' && page.slot === 36}
          >
            Replace…
          </button>
        </>
      ) : (
        <div className="related-missing">No confirmed asset for this role in this ROM.</div>
      )}
    </article>
  );
}
function AssetImage({ asset }: { asset: RelatedAsset }) {
  const pair = useStore((s) => s.snapshot?.pairings.find((p) => p.path === asset.path));
  const revision = useStore((s) => s.snapshot?.history.map((h) => h.applied).join());
  const [url, setUrl] = useState('');
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true,
      object = '';
    void binary({
      path: asset.path!,
      mode: asset.mode === 'cell' ? 'cell-png' : 'png',
      palette: pair?.palette ?? null,
      screen: asset.mode === 'cell' ? (pair?.cells ?? null) : (pair?.screen ?? null),
      texture: asset.index,
      bank: 0,
      start: 0,
      length: 0,
    })
      .then((b) => {
        if (active) {
          object = URL.createObjectURL(new Blob([b], { type: 'image/png' }));
          setUrl(object);
        }
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
      if (object) URL.revokeObjectURL(object);
    };
  }, [asset, pair, revision]);
  return error ? (
    <p role="alert">{error}</p>
  ) : url ? (
    <img src={url} alt={asset.label} />
  ) : (
    <span>Loading image…</span>
  );
}
function AssetAudio({ asset }: { asset: RelatedAsset }) {
  const [url, setUrl] = useState('');
  const [error, setError] = useState('');
  const [sample, setSample] = useState(0);
  const [loading, setLoading] = useState(false);
  useEffect(
    () => () => {
      if (url) URL.revokeObjectURL(url);
    },
    [url],
  );
  return (
    <div className="related-audio">
      {asset.audio_kind === 3 && (
        <label>
          Sample{' '}
          <input
            type="number"
            aria-label="Voice sample"
            min={0}
            value={sample}
            onChange={(e) => {
              setSample(Math.max(0, Number(e.target.value)));
              setUrl('');
            }}
          />
        </label>
      )}
      <button
        disabled={loading}
        onClick={() => {
          setLoading(true);
          setError('');
          void binary({
            path: asset.path!,
            mode: 'audio-wav',
            texture: asset.audio_kind,
            bank: asset.index,
            start: sample,
            length: 0,
            palette: null,
            screen: null,
          })
            .then((b) => setUrl(URL.createObjectURL(new Blob([b], { type: 'audio/wav' }))))
            .catch((e) => setError(String(e)))
            .finally(() => setLoading(false));
        }}
      >
        {loading ? 'Rendering…' : 'Play preview'}
      </button>
      {url && <audio aria-label={asset.label} src={url} controls autoPlay />}
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
function AssetModel({ asset }: { asset: RelatedAsset }) {
  const host = useRef<HTMLDivElement>(null);
  const controller = useRef<{ choose: (index: number) => void; pause: (value: boolean) => void }>();
  const [clips, setClips] = useState<string[]>([]);
  const [clip, setClip] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [error, setError] = useState('');
  const theme = useStore((s) => s.snapshot?.settings.theme);
  const revision = useStore((s) => s.snapshot?.history.map((h) => h.applied).join());
  useEffect(() => {
    if (!host.current) return;
    let active = true,
      frame = 0;
    const element = host.current;
    const scene = new THREE.Scene();
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true });
    } catch {
      setError('WebGL is unavailable.');
      return;
    }
    renderer.setPixelRatio(Math.min(2, devicePixelRatio));
    renderer.setClearColor(theme === 'light' ? '#d6dcdf' : '#20262b');
    renderer.domElement.setAttribute('aria-label', `${asset.label} viewport`);
    element.append(renderer.domElement);
    const camera = new THREE.PerspectiveCamera(45, 1, 0.01, 1000000);
    const controls = new OrbitControls(camera, renderer.domElement);
    scene.add(new THREE.HemisphereLight(0xffffff, 0x6a7380, 3));
    const resize = new ResizeObserver(() => {
      const width = element.clientWidth,
        height = element.clientHeight;
      renderer.setSize(width, height);
      camera.aspect = width / Math.max(1, height);
      camera.updateProjectionMatrix();
    });
    resize.observe(element);
    let mixer: THREE.AnimationMixer | undefined,
      paused = false;
    const load = async (path: string, mode: string) =>
      new GLTFLoader().parseAsync(
        await binary({
          path,
          mode,
          start: 0,
          length: 0,
          palette: null,
          screen: null,
          texture: 0,
          bank: 0,
        }),
        '',
      );
    void Promise.all([
      load(asset.path!, asset.mode === 'collision' ? 'course-glb' : 'glb'),
      inspectScene(asset.path!),
    ])
      .then(async ([gltf, info]) => {
        if (!active) {
          dispose(gltf.scene);
          return;
        }
        scene.add(gltf.scene);
        for (const attachment of asset.attachments) {
          const object = (await load(attachment.path, 'glb')).scene;
          if (!active) {
            dispose(object);
            return;
          }
          // The existing wheel picker writes kart-object coordinates at 16x model units.
          object.position.fromArray(attachment.position).divideScalar(16);
          object.scale.setScalar(attachment.scale);
          scene.add(object);
        }
        const box = new THREE.Box3().setFromObject(scene);
        const center = box.getCenter(new THREE.Vector3());
        const size = Math.max(0.1, box.getSize(new THREE.Vector3()).length());
        if (asset.mode === 'routes') scene.add(overlays(info, Math.max(0.1, size / 100)));
        if (asset.mode === 'collision')
          scene.traverse((o) => {
            if (o instanceof THREE.Mesh) {
              const materials = Array.isArray(o.material) ? o.material : [o.material];
              materials.forEach((m: THREE.Material) => {
                m.transparent = true;
                m.opacity = 0.65;
                m.side = THREE.DoubleSide;
              });
            }
          });
        controls.target.copy(center);
        camera.position.copy(center).add(new THREE.Vector3(size * 0.65, size * 0.55, size * 0.8));
        camera.near = size / 10000;
        camera.far = size * 20;
        camera.updateProjectionMatrix();
        controls.update();
        mixer = new THREE.AnimationMixer(gltf.scene);
        const choose = (index: number) => {
          mixer!.stopAllAction();
          if (gltf.animations[index]) mixer!.clipAction(gltf.animations[index]).reset().play();
        };
        controller.current = {
          choose,
          pause: (value) => {
            paused = value;
          },
        };
        setClips(gltf.animations.map((a) => a.name));
        setClip(0);
        setPlaying(true);
        choose(0);
        renderer.domElement.dataset.loaded = 'true';
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    let last = performance.now();
    const render = (now: number) => {
      if (!paused) mixer?.update(Math.min(0.1, (now - last) / 1000));
      last = now;
      controls.update();
      renderer.render(scene, camera);
      frame = requestAnimationFrame(render);
    };
    frame = requestAnimationFrame(render);
    return () => {
      active = false;
      cancelAnimationFrame(frame);
      resize.disconnect();
      controls.dispose();
      dispose(scene);
      renderer.dispose();
      renderer.forceContextLoss();
      renderer.domElement.remove();
      controller.current = undefined;
    };
  }, [asset, theme, revision]);
  return (
    <>
      <div className="related-model" ref={host} />
      {error && <p role="alert">{error}</p>}
      {!!clips.length && (
        <div className="toolbar">
          <select
            aria-label={`${asset.label} animation`}
            value={clip}
            onChange={(e) => {
              setClip(Number(e.target.value));
              controller.current?.choose(Number(e.target.value));
            }}
          >
            {clips.map((name, i) => (
              <option key={i} value={i}>
                {name}
              </option>
            ))}
          </select>
          <button
            onClick={() => {
              controller.current?.pause(playing);
              setPlaying(!playing);
            }}
          >
            {playing ? 'Pause' : 'Play'}
          </button>
        </div>
      )}
    </>
  );
}

import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { binary, inspectScene } from '../backend';
import { useStore } from '../store';
import { layerDefaults, useScene, type Layer } from '../scene-view';
import { exportSelection } from '../commands';
import type { SceneEntry, SceneInfo } from '../bindings';
import { bytes } from './Properties';

function label(text: string, size: number) {
  const canvas = document.createElement('canvas'); canvas.width = 256; canvas.height = 48;
  const context = canvas.getContext('2d')!;
  context.fillStyle = '#171c20dd'; context.fillRect(0, 0, 256, 48);
  context.fillStyle = '#f0f3ed'; context.font = '24px monospace'; context.textAlign = 'center'; context.fillText(text, 128, 32);
  const map = new THREE.CanvasTexture(canvas);
  const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map, depthTest: false })); sprite.scale.set(size * 5, size, 1); return sprite;
}
const key = (e: SceneEntry) => `${e.section}:${e.index}`;
function overlays(info: SceneInfo, size: number) {
  const root = new THREE.Group();
  const entries = info.entries;
  for (const layer of Object.keys(layerDefaults).filter((k) => k !== 'KCL') as Layer[]) {
    const group = new THREE.Group(); group.name = layer; root.add(group);
    const color = layerDefaults[layer].color;
    const line = (points: THREE.Vector3[], entry?: SceneEntry) => {
      const object = new THREE.Line(new THREE.BufferGeometry().setFromPoints(points), new THREE.LineBasicMaterial({ color, depthTest: false }));
      if (entry) object.userData.entryKey = key(entry); group.add(object); return object;
    };
    for (const e of entries.filter((e) => e.section === layer && e.position)) {
      const position = new THREE.Vector3(...e.position!);
      const marker = new THREE.Mesh(new THREE.BoxGeometry(size, size, size), new THREE.MeshBasicMaterial({ color, wireframe: true, depthTest: false }));
      marker.position.copy(position); marker.userData.entryKey = key(e); group.add(marker);
      if (layer === 'CPOI' && e.end) {
        const end = new THREE.Vector3(...e.end); const up = new THREE.Vector3(0, size * 4, 0);
        line([position, position.clone().add(up), end.clone().add(up), end], e);
      }
      if (layer === 'OBJI') {
        const name = e.fields.find(([name]) => name === 'Object id')?.[1] ?? '?';
        const sprite = label(`OBJI ${name}`, size); sprite.position.copy(position).add(new THREE.Vector3(0, size * 2, 0)); sprite.userData.entryKey = key(e); group.add(sprite);
      }
      if (layer === 'AREA') {
        const data = new DataView(Uint8Array.from(e.raw).buffer); const f = (at: number) => data.getInt32(at, true) / 4096;
        const axis = (at: number, length: number) => new THREE.Vector3(f(at), f(at + 4), f(at + 8)).multiplyScalar(length * 100);
        const x = axis(24, f(12)), y = axis(36, f(16)), z = axis(48, f(20));
        const base = position.clone().addScaledVector(x, -0.5).addScaledVector(z, -0.5);
        const corners = Array.from({ length: 8 }, (_, i) => base.clone().addScaledVector(x, i & 1).addScaledVector(y, (i >> 1) & 1).addScaledVector(z, (i >> 2) & 1));
        corners.forEach((p, i) => { for (const bit of [1, 2, 4]) if (!(i & bit)) line([p, corners[i | bit]], e); });
      }
      if (layer === 'CAME' && e.end) line([position, new THREE.Vector3(...e.end)], e);
    }
    const pathKind = { EPOI: 'EPAT', MEPO: 'MEPA', IPOI: 'IPAT' }[layer as 'EPOI' | 'MEPO' | 'IPOI'];
    if (pathKind) {
      const points = entries.filter((e) => e.section === layer);
      const paths = entries.filter((e) => e.section === pathKind);
      for (const path of paths) {
        const start = Number(path.fields.find(([n]) => n === 'Start index')?.[1]);
        const length = Number(path.fields.find(([n]) => n === 'Length')?.[1]);
        const segment = points.slice(start, start + length).filter((e) => e.position);
        if (segment.length > 1) line(segment.map((e) => new THREE.Vector3(...e.position!)), path);
        const linkCount = layer === 'MEPO' ? 8 : 3;
        for (const id of path.raw.slice(4, 4 + linkCount)) {
          const next = paths[id]; const index = Number(next?.fields.find(([n]) => n === 'Start index')?.[1]);
          if (segment.length && points[index]?.position) line([new THREE.Vector3(...segment.at(-1)!.position!), new THREE.Vector3(...points[index].position!)], path);
        }
      }
    }
  }
  return root;
}
function dispose(object: THREE.Object3D) {
  object.traverse((o) => { if (o instanceof THREE.Mesh || o instanceof THREE.Line || o instanceof THREE.Sprite) {
    if ('geometry' in o) o.geometry.dispose();
    const materials = Array.isArray(o.material) ? o.material : [o.material];
    materials.forEach((m: THREE.Material) => { if ('map' in m && m.map instanceof THREE.Texture) m.map.dispose(); m.dispose(); });
  } });
}
export function SceneViewport() {
  const host = useRef<HTMLDivElement>(null); const counter = useRef<HTMLSpanElement>(null);
  const workspace = useStore((s) => s.workspace); const selected = useStore((s) => s.snapshot?.selected ?? '');
  const files = useStore((s) => s.snapshot?.files); const settings = useStore((s) => s.snapshot?.settings);
  const { mode, grid, set, frameRequest } = useScene();
  const [error, setError] = useState(''); const [loaded, setLoaded] = useState(false);
  const track = workspace === 'Track';
  const format = files?.find((f) => f.path === selected)?.format;
  const valid = track ? ['nkm', 'kcl', 'nsbmd'].includes(format ?? '') : format === 'nsbmd';
  useEffect(() => {
    if (valid || !files?.length) return;
    const f = files.find((f) => f.format === (track ? 'nkm' : 'nsbmd'));
    if (f) void useStore.getState().run({ kind: 'Select', path: f.path });
  }, [valid, files, track]);
  useEffect(() => {
    if (!valid || !host.current) return;
    let cancelled = false; let animationFrame = 0; let content: THREE.Group | null = null; let markerRoot: THREE.Group | null = null;
    let mixer: THREE.AnimationMixer | null = null; let clips: THREE.AnimationClip[] = []; let activeAnimation = -2; let activeFrameRequest = frameRequest;
    let renderer: THREE.WebGLRenderer;
    try { renderer = new THREE.WebGLRenderer({ antialias: true }); } catch { setError('WebGL is unavailable. Enable graphics acceleration to view 3D models.'); return; }
    setLoaded(false); setError(''); useScene.setState({ info: null, selected: '', animation: -1, frame: 0, playing: false });
    const element = host.current; renderer.setPixelRatio(Math.min(devicePixelRatio, 2)); renderer.setClearColor(settings?.theme === 'light' ? '#d6dcdf' : '#20262b');
    renderer.domElement.setAttribute('aria-label', '3D viewport'); renderer.domElement.tabIndex = 0; element.append(renderer.domElement);
    const scene = new THREE.Scene(); const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1000000);
    const controls = new OrbitControls(camera, renderer.domElement); controls.enableDamping = true;
    const mktb = settings?.navigation === 'MKTB'; controls.mouseButtons = { LEFT: mktb ? THREE.MOUSE.ROTATE : undefined, MIDDLE: THREE.MOUSE.ROTATE, RIGHT: THREE.MOUSE.PAN };
    controls.rotateSpeed = settings?.camera_speed ?? 1; controls.panSpeed = settings?.camera_speed ?? 1; controls.zoomSpeed = settings?.camera_speed ?? 1;
    scene.add(new THREE.HemisphereLight(0xffffff, 0x8899aa, 2)); const light = new THREE.DirectionalLight(0xffffff, 2); light.position.set(1, 3, 2); scene.add(light);
    const gridObject = new THREE.GridHelper(2000, 40, 0x617477, 0x3e4a50); scene.add(gridObject);
    const resize = () => { const w = Math.max(1, element.clientWidth), h = Math.max(1, element.clientHeight); renderer.setSize(w, h); camera.aspect = w / h; camera.updateProjectionMatrix(); };
    const observer = new ResizeObserver(resize); observer.observe(element); resize();
    const frame = () => {
      let box = new THREE.Box3(); const state = useScene.getState(); const e = state.info?.entries.find((e) => key(e) === state.selected);
      if (track && e?.position) { const p = new THREE.Vector3(...e.position); box.setFromCenterAndSize(p, new THREE.Vector3(100, 100, 100)); }
      else if (content) box.setFromObject(content);
      if (box.isEmpty()) return; const center = box.getCenter(new THREE.Vector3()); const radius = Math.max(box.getSize(new THREE.Vector3()).length() / 2, 1);
      camera.position.copy(center).add(new THREE.Vector3(1.1, 0.85, 1.25).multiplyScalar(radius * 1.5)); controls.target.copy(center); camera.near = Math.max(0.01, radius / 10000); camera.far = radius * 1000; camera.updateProjectionMatrix(); controls.update();
    };
    const keydown = (e: KeyboardEvent) => { if (e.key.toLowerCase() === 'f' || e.code === 'NumpadDecimal') { e.preventDefault(); frame(); } };
    renderer.domElement.addEventListener('keydown', keydown);
    const raycaster = new THREE.Raycaster(); let down = new THREE.Vector2();
    const pointerdown = (e: PointerEvent) => { down = new THREE.Vector2(e.clientX, e.clientY); };
    const pointerup = (e: PointerEvent) => {
      if (e.button !== 0 || down.distanceTo(new THREE.Vector2(e.clientX, e.clientY)) > 4 || !markerRoot) return;
      const rect = renderer.domElement.getBoundingClientRect(); raycaster.setFromCamera(new THREE.Vector2((e.clientX - rect.left) / rect.width * 2 - 1, -(e.clientY - rect.top) / rect.height * 2 + 1), camera);
      const hit = raycaster.intersectObject(markerRoot, true).find((hit) => {
        const layer = hit.object.parent?.name as Layer; return hit.object.userData.entryKey && useScene.getState().layers[layer]?.visible && !useScene.getState().layers[layer]?.locked;
      });
      if (hit) useScene.setState({ selected: hit.object.userData.entryKey as string });
    };
    renderer.domElement.addEventListener('pointerdown', pointerdown); renderer.domElement.addEventListener('pointerup', pointerup);
    void Promise.all([inspectScene(selected), binary({ path: selected, mode: track ? 'course-glb' : 'glb', start: 0, length: 0, palette: null, screen: null, texture: 0, bank: 0 })]).then(async ([info, data]) => {
      const gltf = await new GLTFLoader().parseAsync(data, ''); if (cancelled) { dispose(gltf.scene); return; }
      content = gltf.scene; scene.add(content); clips = gltf.animations; mixer = new THREE.AnimationMixer(content);
      if (track) { const box = new THREE.Box3().setFromObject(content); const size = Math.max(10, box.getSize(new THREE.Vector3()).length() / 75); markerRoot = overlays(info, size); scene.add(markerRoot); }
      useScene.setState({ info }); setLoaded(true); frame();
    }).catch((e: unknown) => { if (!cancelled) setError(String(e)); });
    let last = performance.now(), fpsTime = last, frames = 0; const originalMaps = new Map<THREE.Material, THREE.Texture | null>();
    const render = (time: number) => {
      const dt = Math.min((time - last) / 1000, 0.1); last = time; const state = useScene.getState(); gridObject.visible = state.grid;
      if (state.frameRequest !== activeFrameRequest) { activeFrameRequest = state.frameRequest; frame(); }
      if (content) content.traverse((o) => { if (o instanceof THREE.Mesh) {
        const collision = o.name.startsWith('collision:'); o.visible = !collision || (track && state.layers.KCL.visible);
        const materials = Array.isArray(o.material) ? o.material : [o.material];
        for (const m of materials) { if (m instanceof THREE.MeshStandardMaterial) {
          if (!originalMaps.has(m)) originalMaps.set(m, m.map); const map = state.mode === 'textured' ? originalMaps.get(m)! : null;
          if (m.map !== map) { m.map = map; m.needsUpdate = true; } m.wireframe = state.mode === 'wireframe';
        } }
      } });
      if (markerRoot) for (const group of markerRoot.children) { const l = state.layers[group.name as Layer]; group.visible = l.visible; group.traverse((o) => { if (o instanceof THREE.Mesh || o instanceof THREE.Line) { const m = o.material as THREE.MeshBasicMaterial; m.color.set(l.color); m.opacity = l.locked ? 0.35 : 1; m.transparent = l.locked; } }); }
      if (mixer) {
        if (activeAnimation !== state.animation) { mixer.stopAllAction(); content?.traverse((o) => { if (o instanceof THREE.SkinnedMesh) o.skeleton.pose(); }); activeAnimation = state.animation; if (clips[activeAnimation]) mixer.clipAction(clips[activeAnimation]).play(); }
        if (clips[activeAnimation]) { const duration = clips[activeAnimation].duration; if (state.playing) { mixer.update(dt); const f = Math.floor((mixer.time % Math.max(duration, 0.01)) * 60); if (f !== state.frame) useScene.setState({ frame: f }); } else mixer.setTime(state.frame / 60); }
      }
      controls.update(); renderer.render(scene, camera); frames++;
      if (time - fpsTime > 500 && counter.current) { counter.current.textContent = `${Math.round(frames * 1000 / (time - fpsTime))} FPS · ${renderer.info.render.triangles.toLocaleString()} tris`; frames = 0; fpsTime = time; }
      animationFrame = requestAnimationFrame(render);
    }; animationFrame = requestAnimationFrame(render);
    return () => { cancelled = true; cancelAnimationFrame(animationFrame); observer.disconnect(); controls.dispose(); mixer?.stopAllAction(); renderer.domElement.removeEventListener('keydown', keydown); renderer.domElement.removeEventListener('pointerdown', pointerdown); renderer.domElement.removeEventListener('pointerup', pointerup); dispose(scene); renderer.dispose(); renderer.domElement.remove(); };
    // View options are read by the renderer without rebuilding the scene.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected, valid, track, settings?.theme, settings?.navigation, settings?.camera_speed]);
  return <div className="panel scene-panel">
    <div className="scene-toolbar"><select aria-label={track ? 'Course' : 'Model file'} value={valid ? selected : ''} onChange={(e) => void useStore.getState().run({ kind: 'Select', path: e.target.value })}><option value="" disabled>Select {track ? 'course' : 'model'}</option>{files?.filter((f) => f.format === (track ? 'nkm' : 'nsbmd')).map((f) => <option key={f.path} value={f.path}>{f.path}</option>)}</select>
      <select aria-label="Shading" value={mode} onChange={(e) => set({ mode: e.target.value as typeof mode })}><option value="textured">Textured</option><option value="solid">Solid</option><option value="wireframe">Wireframe</option></select>
      <label><input type="checkbox" checked={grid} onChange={(e) => set({ grid: e.target.checked })} />Grid</label><button onClick={() => set({ frameRequest: frameRequest + 1 })}>Frame selected · F</button></div>
    <div className="viewport-host" ref={host} data-testid="viewport" data-loaded={loaded}>{error && <div className="viewport-message" role="alert">{error}</div>}{!valid && <div className="viewport-message">Open a {track ? 'course NKM' : 'NSBMD model'} from Explorer.</div>}<span className="viewport-counter" ref={counter} /><span className="viewport-hint">{settings?.navigation ?? 'Blender'} · MMB orbit · RMB pan · Wheel zoom</span><span className="viewport-badge">{track ? 'COURSE' : 'MODEL'} / READ ONLY</span></div>
  </div>;
}
export function SceneOutliner() {
  const { info, selected, set } = useScene(); const track = useStore((s) => s.workspace === 'Track');
  return <div className="panel scene-outliner"><div className="section-label">{track ? 'COURSE CONTENTS' : 'MODEL CONTENTS'}</div>{!info && <p className="inset muted">Choose a file in the viewport.</p>}{track ? [...new Set(info?.entries.map((e) => e.section))].map((section) => <details key={section} open><summary>{section} <span>{info?.entries.filter((e) => e.section === section).length}</span></summary>{info?.entries.filter((e) => e.section === section).map((e) => <button key={key(e)} className={`outliner-entry ${selected === key(e) ? 'selected' : ''}`} onClick={() => set({ selected: key(e) })} onDoubleClick={() => set({ selected: key(e), frameRequest: useScene.getState().frameRequest + 1 })}>{section} {e.index}{section === 'OBJI' ? ` · id ${e.fields.find(([n]) => n === 'Object id')?.[1]}` : ''}</button>)}</details>) : <><details open><summary>Bones · {info?.bones.length ?? 0}</summary>{info?.bones.map((b, i) => <div className="outliner-entry" key={i}>{i} · {b}</div>)}</details><details open><summary>Materials · {info?.materials.length ?? 0}</summary>{info?.materials.map((m, i) => <div className="outliner-entry" key={i}>{m}</div>)}</details></>}</div>;
}
export function SceneProperties() {
  const { info, selected } = useScene(); const entry = info?.entries.find((e) => key(e) === selected); const budget = info?.budget;
  return <div className="panel properties"><div className="section-label">DS BUDGET · GEOMETRY ESTIMATE</div>{budget && <><dl>{[["Polygons (triangulated)", `${budget.polygons} / 2,048`],["Vertices",`${budget.vertices} / 6,144`],["Texture VRAM",`${bytes(budget.texture_bytes)} / 512 KiB`],["Palette VRAM",`${bytes(budget.palette_bytes)} / 96 KiB`],["Matrix slots",`${budget.matrices} / 32`]].map(([label, value]) => <div className="budget-row" key={label}><dt>{label}</dt><dd>{value}</dd></div>)}</dl><p className="inset muted">Per model set. Frame-wide budgets depend on the game scene.</p>{budget.reasons.map((r) => <p className="issue" key={r}>{r}</p>)}</>}<div className="section-label">{entry ? `${entry.section} ${entry.index} · READ ONLY` : 'SELECTION'}</div>{entry ? <><dl>{entry.fields.map(([name, value]) => <div className="budget-row" key={name}><dt>{name}</dt><dd>{value}</dd></div>)}</dl><details className="inset"><summary>All record bytes</summary><code className="record-bytes">{entry.raw.map((n) => n.toString(16).padStart(2, '0')).join(' ')}</code></details></> : <p className="inset muted">Select an entry in the Outliner or viewport.</p>}<div className="section-label">VALIDATION</div>{info?.issues.length ? info.issues.map((r, i) => <p className="issue" key={i}>{r}</p>) : <p className="inset muted">No course issues reported.</p>}<div className="inset"><button disabled={!info?.model_path} onClick={() => { const p = info?.model_path; if (p) void useStore.getState().run({ kind: 'Select', path: p }).then(() => exportSelection('glb')); }}>Export GLB…</button></div></div>;
}
export function Overlays() {
  const { layers, set, info } = useScene();
  return <div className="panel overlay-panel">{(Object.keys(layers) as Layer[]).map((key) => { const l = layers[key]; const update = (v: Partial<typeof l>) => set({ layers: { ...layers, [key]: { ...l, ...v } } }); return <div className="overlay-row" key={key}><input aria-label={`Show ${l.name}`} type="checkbox" checked={l.visible} onChange={(e) => update({ visible: e.target.checked })} /><span>{l.name}</span><input aria-label={`${l.name} colour`} type="color" value={l.color} onChange={(e) => update({ color: e.target.value })} /><button className={l.locked ? 'active' : ''} aria-label={`Lock ${l.name}`} aria-pressed={l.locked} onClick={() => update({ locked: !l.locked })}>{l.locked ? 'Locked' : 'Lock'}</button></div>; })}<details className="inset" open><summary>Collision legend (type bits 0–4)</summary>{info?.collision_types.map((t) => { const h = t * 2.3999631; const rgb = [0, 2.094, 4.189].map((v) => Math.round((0.5 + 0.35 * Math.sin(h + v)) * 255)); return <div className="legend-row" key={t}><i style={{ background: `rgb(${rgb.join(',')})` }} />Type {t}</div>; })}</details></div>;
}
export function Timeline() {
  const { info, animation, frame, playing, set } = useScene();
  const frames = Number(info?.animations[animation]?.match(/(\d+) frames/)?.[1] ?? 1);
  return <div className="panel timeline"><div className="scene-toolbar"><label>Animation <select aria-label="Animation" value={animation} onChange={(e) => set({ animation: Number(e.target.value), frame: 0, playing: false })}><option value={-1}>Rest pose</option>{info?.animations.map((a, i) => <option key={i} value={i}>{a}</option>)}</select></label><button disabled={animation < 0} onClick={() => set({ playing: !playing })}>{playing ? 'Pause animation' : 'Play animation'}</button><span>{frame} / {frames - 1} · 60 fps</span></div><input aria-label="Animation frame" type="range" min={0} max={frames - 1} value={frame} disabled={animation < 0} onChange={(e) => set({ frame: Number(e.target.value), playing: false })} /></div>;
}

import * as THREE from 'three';
import type { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { ModelAction, Operation, Selection } from './bindings';
import { useScene } from './scene-view';
import { useStore } from './store';
import { runModel, selectPart } from './model-actions';
import {
  allSelection,
  combine,
  emptySelection,
  fromVertices,
  previewEdit,
  selectedVertices,
  topology,
} from './mesh-selection';

export function createModelEditor(
  canvas: HTMLCanvasElement,
  camera: THREE.PerspectiveCamera,
  content: THREE.Group,
  scene: THREE.Scene,
  controls: OrbitControls,
) {
  const meshes = new Map<number, THREE.Mesh>();
  content.traverse((o) => {
    if (o instanceof THREE.Mesh && typeof o.userData.part === 'number' && !o.userData.attachment)
      meshes.set(o.userData.part as number, o);
  });
  const originals = new Map([...meshes].map(([i, m]) => [i, m.geometry]));
  const ray = new THREE.Raycaster(),
    overlay = new THREE.Group();
  scene.add(overlay);
  const joint = new THREE.Mesh(
    new THREE.SphereGeometry(1, 12, 8),
    new THREE.MeshBasicMaterial({ color: 0xffa64d, depthTest: false }),
  );
  joint.renderOrder = 10;
  const axes = new THREE.AxesHelper(3);
  joint.add(axes);
  joint.visible = false;
  scene.add(joint);
  const box = document.createElement('div');
  box.className = 'selection-box';
  box.hidden = true;
  canvas.parentElement!.append(box);
  let last = new THREE.Vector2(canvas.clientWidth / 2, canvas.clientHeight / 2),
    down = last.clone(),
    boxStart: THREE.Vector2 | null = null,
    signature = '';
  let pending: {
    key: string;
    axis: string;
    text: string;
    amount: number;
    start: THREE.Vector2;
    actions: ModelAction[];
    viewOffset: THREE.Vector3;
  } | null = null;
  let consumed = false;
  const point = (e: PointerEvent) => {
    const r = canvas.getBoundingClientRect();
    return new THREE.Vector2(e.clientX - r.left, e.clientY - r.top);
  };
  const project = (p: THREE.Vector3, m: THREE.Mesh) =>
    p.clone().applyMatrix4(m.matrixWorld).project(camera);
  const cast = () => {
    ray.setFromCamera(
      new THREE.Vector2(
        (last.x / canvas.clientWidth) * 2 - 1,
        (-last.y / canvas.clientHeight) * 2 + 1,
      ),
      camera,
    );
  };
  const error = (message: string) => useStore.getState().set({ error: message });
  const clearOverlay = () => {
    for (const o of [...overlay.children]) {
      overlay.remove(o);
      if (o instanceof THREE.Mesh || o instanceof THREE.Points || o instanceof THREE.LineSegments) {
        o.geometry.dispose();
        (o.material as THREE.Material).dispose();
      }
    }
  };
  const restore = () => {
    for (const [i, m] of meshes) {
      const original = originals.get(i)!;
      if (m.geometry !== original) {
        m.geometry.dispose();
        m.geometry = original;
      }
    }
  };
  const cancel = () => {
    restore();
    pending = null;
    controls.enabled = true;
    useScene.setState({ modal: '', liveTriangles: null, liveVertices: null, editMenu: '' });
    signature = '';
  };
  function selectAll(none = false) {
    const s = useScene.getState();
    if (s.editMode) {
      const elements: Record<number, Selection> = {};
      if (!none)
        for (const [i, g] of originals)
          if (s.parts.includes(i)) elements[i] = allSelection(g, s.elementMode);
      useScene.setState({ elements });
    } else useScene.setState({ parts: none ? [] : [...meshes.keys()] });
  }
  function convertMode(mode: 'vertex' | 'edge' | 'face') {
    const s = useScene.getState(),
      elements: Record<number, Selection> = {};
    for (const [i, sel] of Object.entries(s.elements)) {
      const g = originals.get(Number(i));
      if (g) elements[Number(i)] = fromVertices(g, mode, selectedVertices(g, sel));
    }
    useScene.setState({ elementMode: mode, elements });
  }
  const chosen = () => {
    const s = useScene.getState();
    return [...meshes].filter(([i]) =>
      s.editMode
        ? !!s.elements[i] && selectedVertices(originals.get(i)!, s.elements[i]).length > 0
        : s.parts.includes(i),
    );
  };
  function applyOperation(op: Operation, deleteMode?: string) {
    const s = useScene.getState();
    const actions: ModelAction[] = chosen().map(([i]) => ({
      kind: 'Edit',
      part: i,
      selection: deleteMode
        ? fromVertices(
            originals.get(i)!,
            deleteMode,
            selectedVertices(originals.get(i)!, s.elements[i]),
          )
        : s.elements[i],
      operation: op,
    }));
    if (!actions.length) {
      error('Select mesh elements first');
      return;
    }
    cancel();
    void runModel(actions);
  }
  function start(key: string) {
    const s = useScene.getState();
    if (!useStore.getState().snapshot?.project_path || useStore.getState().busy) return;
    if (!chosen().length) {
      error('Select parts or mesh elements first');
      return;
    }
    if ((key === 'e' || key === 'i') && !s.editMode) return;
    if (key === 'e' && s.elementMode === 'vertex') {
      error('Select edges or faces to extrude');
      return;
    }
    if (key === 'i' && s.elementMode !== 'face') {
      error('Select faces to inset');
      return;
    }
    pending = {
      key,
      axis: '',
      text: '',
      amount: key === 's' ? 1 : 0,
      start: last.clone(),
      actions: [],
      viewOffset: new THREE.Vector3(),
    };
    controls.enabled = false;
    useScene.setState({ playing: false, animation: -1, gizmo: 'off' });
    preview();
  }
  function preview() {
    if (!pending) return;
    const p = pending,
      s = useScene.getState();
    restore();
    const selected = chosen(),
      pivot = new THREE.Vector3();
    let count = 0;
    for (const [i] of selected) {
      const t = topology(originals.get(i)!);
      const ids = s.editMode
        ? selectedVertices(originals.get(i)!, s.elements[i])
        : t.positions.map((_, i) => i);
      const seen = new Set<string>();
      for (const id of ids) {
        if (seen.has(t.keys[id])) continue;
        seen.add(t.keys[id]);
        pivot.add(t.positions[id]);
        count++;
      }
    }
    pivot.divideScalar(Math.max(1, count));
    const axis = p.axis
      ? new THREE.Vector3(
          ...({ x: [1, 0, 0], y: [0, 1, 0], z: [0, 0, 1] }[p.axis] as [number, number, number]),
        )
      : p.key === 'r'
        ? camera.getWorldDirection(new THREE.Vector3()).negate()
        : new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion);
    const amount = p.text && Number.isFinite(Number(p.text)) ? Number(p.text) : p.amount;
    const q = new THREE.Quaternion().setFromAxisAngle(axis, (amount * Math.PI) / 180);
    const scale = new THREE.Vector3(1, 1, 1);
    if (p.key === 's') {
      if (p.axis) scale.setComponent('xyz'.indexOf(p.axis), amount);
      else scale.setScalar(amount);
    }
    const operation: Operation =
      p.key === 'e'
        ? { kind: 'Extrude', amount }
        : p.key === 'i'
          ? { kind: 'Inset', amount: Math.max(0, amount) }
          : {
              kind: 'Transform',
              translation:
                p.key === 'g'
                  ? !p.axis && !p.text
                    ? p.viewOffset.toArray()
                    : axis.multiplyScalar(amount).toArray()
                  : [0, 0, 0],
              rotation: p.key === 'r' ? q.toArray() : [0, 0, 0, 1],
              scale: scale.toArray(),
              pivot: pivot.toArray(),
            };
    p.actions = selected.map(([i, m]) => {
      const selection = s.editMode ? s.elements[i] : allSelection(originals.get(i)!, 'vertex');
      m.geometry = previewEdit(originals.get(i)!, selection, operation);
      return { kind: 'Edit', part: i, selection, operation };
    });
    let total = 0;
    for (const m of meshes.values())
      total += (m.geometry.index?.count ?? m.geometry.getAttribute('position').count) / 3;
    useScene.setState({
      modal: `${{ g: 'Move', r: 'Rotate', s: 'Scale', e: 'Extrude', i: 'Inset' }[p.key]} ${p.axis.toUpperCase() || 'view'}: ${p.text || amount.toFixed(3)} · Enter / LMB confirm · Esc / RMB cancel`,
      liveTriangles: total,
      liveVertices: [...meshes.values()].reduce(
        (n, m) => n + m.geometry.getAttribute('position').count,
        0,
      ),
    });
    signature = '';
  }
  function commit() {
    if (!pending) return;
    const actions = pending.actions;
    if (
      (useScene.getState().liveTriangles ?? 0) > 2048 ||
      (useScene.getState().liveVertices ?? 0) > 6144
    ) {
      error('DS polygon budget exceeded; reduce geometry before confirming');
      return;
    }
    cancel();
    void runModel(actions);
  }
  function linked() {
    const s = useScene.getState();
    cast();
    const hit = ray
      .intersectObjects([...meshes].filter(([i]) => s.parts.includes(i)).map(([, m]) => m))
      .find((h) => h.faceIndex != null);
    const i = hit?.object.userData.part as number | undefined;
    if (i === undefined) return;
    const g = originals.get(i)!,
      t = topology(g),
      seed = hit!.faceIndex! * 3,
      vertices = new Set(t.indices.slice(seed, seed + 3).map((i) => t.logical[i]));
    let changed = true;
    while (changed) {
      changed = false;
      for (let f = 0; f < t.indices.length; f += 3) {
        const tri = t.indices.slice(f, f + 3).map((i) => t.logical[i]);
        if (tri.some((i) => vertices.has(i)))
          for (const v of tri)
            if (!vertices.has(v)) {
              vertices.add(v);
              changed = true;
            }
      }
    }
    useScene.setState({
      elements: {
        ...s.elements,
        [i]: combine(
          s.elements[i] ?? emptySelection(s.elementMode),
          fromVertices(g, s.elementMode, [...vertices]),
        ),
      },
    });
  }
  function command(key: string) {
    const s = useScene.getState();
    if (key === 'tab') {
      cancel();
      useScene.setState({
        editMode: !s.editMode,
        playing: false,
        animation: -1,
        elements: {},
        gizmo: 'off',
      });
      return;
    }
    if (key === 'all' || key === 'none') {
      selectAll(key === 'none');
      return;
    }
    if (['1', '2', '3'].includes(key) && s.editMode) {
      convertMode((['vertex', 'edge', 'face'] as const)[Number(key) - 1]);
      return;
    }
    if (key === 'b') {
      useScene.setState({ boxSelect: !s.boxSelect });
      return;
    }
    if (key === 'l' && s.editMode) {
      linked();
      return;
    }
    if (key === 'x' || key === 'delete') {
      if (s.editMode) useScene.setState({ editMenu: 'delete' });
      else
        void runModel(
          s.parts.map((part) => ({ kind: 'Delete', part, selection: { kind: 'Part' } })),
        );
      return;
    }
    if (key === 'm' && s.editMode) {
      useScene.setState({ editMenu: 'merge' });
      return;
    }
    if (key.startsWith('delete-')) {
      applyOperation(
        { kind: 'Delete' },
        { vertices: 'vertex', edges: 'edge', faces: 'face' }[key.slice(7)],
      );
      return;
    }
    if (key === 'merge-center') {
      applyOperation({ kind: 'Merge' });
      return;
    }
    if (['g', 'r', 's', 'e', 'i'].includes(key)) start(key);
  }
  const onCommand = (e: Event) => command((e as CustomEvent<string>).detail);
  const keydown = (e: KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey) {
      if (!pending && e.key.toLowerCase() !== 'l') return;
    }
    const key = e.key.toLowerCase();
    if (!pending && e.altKey && key === 'z') {
      e.preventDefault();
      useScene.setState({ xray: !useScene.getState().xray });
      return;
    }
    if (pending) {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (key === 'escape') {
        cancel();
        return;
      }
      if (key === 'enter') {
        commit();
        return;
      }
      if (['x', 'y', 'z'].includes(key) && ['g', 'r', 's'].includes(pending.key)) {
        pending.axis = key;
        preview();
        return;
      }
      if (/^[0-9.-]$/.test(key)) {
        pending.text += key;
        preview();
        return;
      }
      if (key === 'backspace') {
        pending.text = pending.text.slice(0, -1);
        preview();
      }
      return;
    }
    if (key === 'escape') {
      cancel();
      useScene.setState({ boxSelect: false });
      return;
    }
    if (
      ['tab', '1', '2', '3', 'a', 'b', 'l', 'g', 'r', 's', 'e', 'i', 'x', 'delete', 'm'].includes(
        key,
      )
    ) {
      e.preventDefault();
      e.stopPropagation();
      command(key === 'a' ? (e.altKey ? 'none' : 'all') : key);
    }
  };
  const pointerdown = (e: PointerEvent) => {
    last = point(e);
    down = last.clone();
    consumed = false;
    if (pending) {
      e.preventDefault();
      e.stopImmediatePropagation();
      consumed = true;
      if (e.button === 2) cancel();
      else if (e.button === 0) commit();
      return;
    }
    const s = useScene.getState();
    if (s.boxSelect && e.button === 0) {
      boxStart = last.clone();
      controls.enabled = false;
      box.hidden = false;
      e.stopImmediatePropagation();
    }
  };
  const pointermove = (e: PointerEvent) => {
    last = point(e);
    if (boxStart) {
      box.style.left = Math.min(last.x, boxStart.x) + 'px';
      box.style.top = Math.min(last.y, boxStart.y) + 'px';
      box.style.width = Math.abs(last.x - boxStart.x) + 'px';
      box.style.height = Math.abs(last.y - boxStart.y) + 'px';
    }
    if (pending) {
      const extent = Math.max(0.01, camera.position.distanceTo(controls.target));
      const units =
        ((2 * extent * Math.tan(THREE.MathUtils.degToRad(camera.fov / 2))) / canvas.clientHeight) *
        (e.shiftKey ? 0.1 : 1);
      pending.viewOffset
        .set(last.x - pending.start.x, pending.start.y - last.y, 0)
        .multiplyScalar(units)
        .applyQuaternion(camera.quaternion);
      if (e.ctrlKey)
        pending.viewOffset.set(
          Math.round(pending.viewOffset.x),
          Math.round(pending.viewOffset.y),
          Math.round(pending.viewOffset.z),
        );
      let delta = (last.x - pending.start.x - (last.y - pending.start.y)) / canvas.clientWidth;
      if (e.shiftKey) delta *= 0.1;
      let value =
        pending.key === 's' ? 1 + delta * 2 : pending.key === 'r' ? delta * 180 : delta * extent;
      if (e.ctrlKey) {
        const step = pending.key === 'r' ? 5 : pending.key === 's' ? 0.1 : 1;
        value = Math.round(value / step) * step;
      }
      pending.amount = value;
      preview();
    }
  };
  const pointerup = (e: PointerEvent) => {
    if (consumed) {
      consumed = false;
      return;
    }
    if (e.button !== 0) return;
    const s = useScene.getState();
    if (s.pickWheels || s.rigBrush) return;
    last = point(e);
    if (boxStart) {
      const min = boxStart.clone().min(last),
        max = boxStart.clone().max(last),
        elements: Record<number, Selection> = e.shiftKey || e.ctrlKey ? { ...s.elements } : {},
        parts = e.shiftKey || e.ctrlKey ? [...s.parts] : [];
      for (const [i, m] of meshes) {
        if (s.editMode && !s.parts.includes(i)) continue;
        const t = topology(originals.get(i)!);
        const ids = t.positions
          .map((p, i) => ({ p: project(p, m), i }))
          .filter(({ p, i: vertex }) => {
            const x = ((p.x + 1) * canvas.clientWidth) / 2,
              y = ((1 - p.y) * canvas.clientHeight) / 2;
            if (!(p.z >= -1 && p.z <= 1 && x >= min.x && x <= max.x && y >= min.y && y <= max.y))
              return false;
            if (!s.editMode || s.xray) return true;
            ray.setFromCamera(new THREE.Vector2(p.x, p.y), camera);
            const hit = ray.intersectObjects([...meshes.values()])[0];
            const world = t.positions[vertex].clone().applyMatrix4(m.matrixWorld);
            return (
              !hit ||
              hit.distance + Math.max(0.0001, camera.position.distanceTo(world) * 0.002) >=
                camera.position.distanceTo(world)
            );
          })
          .map((v) => v.i);
        if (ids.length) {
          if (s.editMode)
            elements[i] = combine(
              elements[i] ?? emptySelection(s.elementMode),
              fromVertices(originals.get(i)!, s.elementMode, ids),
            );
          else if (!parts.includes(i)) parts.push(i);
        }
      }
      useScene.setState({ elements, parts: s.editMode ? s.parts : parts, boxSelect: false });
      boxStart = null;
      box.hidden = true;
      controls.enabled = true;
      return;
    }
    if (down.distanceTo(last) > 4 || s.gizmo !== 'off') return;
    cast();
    if (joint.visible && ray.intersectObject(joint).length) {
      useScene.setState({ highlight: 'bone' });
      return;
    }
    const hit = ray
      .intersectObjects(
        [...meshes].filter(([i]) => !s.editMode || s.parts.includes(i)).map(([, m]) => m),
      )
      .find((h) => h.faceIndex != null);
    if (!hit) {
      if (!e.shiftKey && !e.ctrlKey)
        useScene.setState(s.editMode ? { elements: {} } : { parts: [] });
      return;
    }
    const i = hit.object.userData.part as number;
    if (!s.editMode) {
      selectPart(i, e.shiftKey || e.ctrlKey || e.metaKey);
      useScene.setState({ triangle: hit.faceIndex ?? 0 });
      return;
    }
    const g = originals.get(i)!,
      t = topology(g),
      tri = t.indices.slice(hit.faceIndex! * 3, hit.faceIndex! * 3 + 3),
      local = (hit.object as THREE.Mesh).worldToLocal(hit.point.clone());
    const selection = emptySelection(s.elementMode);
    if (s.elementMode === 'face') selection.faces = [hit.faceIndex!];
    if (s.elementMode === 'vertex')
      selection.vertices = [
        t.logical[
          tri.reduce((a, b) =>
            local.distanceToSquared(t.positions[a]) < local.distanceToSquared(t.positions[b])
              ? a
              : b,
          )
        ],
      ];
    if (s.elementMode === 'edge') {
      let best: [number, number] = [tri[0], tri[1]],
        distance = Infinity;
      for (let k = 0; k < 3; k++) {
        const a = tri[k],
          b = tri[(k + 1) % 3],
          line = new THREE.Line3(t.positions[a], t.positions[b]),
          closest = line.closestPointToPoint(local, true, new THREE.Vector3()),
          d = local.distanceToSquared(closest);
        if (d < distance) {
          distance = d;
          best = [t.logical[a], t.logical[b]];
        }
      }
      best.sort((a, b) => a - b);
      selection.edges = [best];
    }
    const add = e.shiftKey || e.ctrlKey || e.metaKey;
    useScene.setState({
      part: i,
      highlight: 'none',
      elements: {
        ...(add ? s.elements : {}),
        [i]: add
          ? combine(s.elements[i] ?? emptySelection(s.elementMode), selection, true)
          : selection,
      },
    });
  };
  function update() {
    const s = useScene.getState();
    canvas.dataset.editMode = String(s.editMode);
    canvas.dataset.selection = JSON.stringify(s.editMode ? s.elements : s.parts);
    canvas.dataset.highlight =
      s.highlight === 'none'
        ? 'none'
        : `${s.highlight}:${s.highlight === 'bone' ? s.bone : s.material}`;
    joint.visible = s.highlight === 'bone' && !!s.info?.bone_details[s.bone];
    if (joint.visible) {
      joint.position.fromArray(s.info!.bone_details[s.bone].position);
      const size = Math.max(0.005, camera.position.distanceTo(joint.position) / 90);
      joint.scale.setScalar(size);
    }
    const next = JSON.stringify([
      s.elements,
      s.parts,
      s.editMode,
      s.highlight,
      s.bone,
      s.material,
      s.modal,
    ]);
    if (signature === next) return;
    signature = next;
    clearOverlay();
    for (const [i, m] of meshes) {
      const g = m.geometry,
        t = topology(g);
      let selected: number[] = [];
      if (s.highlight === 'bone') {
        const js = g.getAttribute('skinIndex'),
          ws = g.getAttribute('skinWeight');
        if (js && ws)
          selected = t.positions
            .map((_, i) => i)
            .filter((i) =>
              [0, 1, 2, 3].some((k) => js.array[i * 4 + k] === s.bone && ws.array[i * 4 + k] > 0),
            );
      } else if (s.highlight === 'material') {
        if (s.info?.parts[i]?.material === s.material) selected = t.positions.map((_, i) => i);
      } else if (s.editMode && s.elements[i]) selected = selectedVertices(g, s.elements[i]);
      else if (!s.editMode && s.parts.includes(i)) selected = t.positions.map((_, i) => i);
      if (!selected.length) continue;
      const color = 0xffaa50;
      const geo = new THREE.BufferGeometry().setFromPoints(
        selected.map((v) => t.positions[v].clone().applyMatrix4(m.matrixWorld)),
      );
      const points = new THREE.Points(
        geo,
        new THREE.PointsMaterial({
          color,
          size: s.editMode && s.elementMode === 'vertex' ? 7 : 3,
          sizeAttenuation: false,
          depthTest: false,
        }),
      );
      points.renderOrder = 8;
      overlay.add(points);
      const chosen = new Set(selected),
        lines: THREE.Vector3[] = [];
      for (const [a, b] of t.edges)
        if (chosen.has(a) && chosen.has(b))
          lines.push(
            t.positions[a].clone().applyMatrix4(m.matrixWorld),
            t.positions[b].clone().applyMatrix4(m.matrixWorld),
          );
      const wire = new THREE.LineSegments(
        new THREE.BufferGeometry().setFromPoints(lines),
        new THREE.LineBasicMaterial({ color, depthTest: false, transparent: true, opacity: 0.7 }),
      );
      wire.renderOrder = 7;
      overlay.add(wire);
      const faces: number[] =
        s.highlight === 'material'
          ? Array.from({ length: t.indices.length / 3 }, (_, i) => i)
          : s.editMode && s.elementMode === 'face'
            ? (s.elements[i]?.faces ?? [])
            : [];
      if (faces.length) {
        const p = faces
          .flatMap((f) => t.indices.slice(f * 3, f * 3 + 3))
          .map((v) => t.positions[v].clone().applyMatrix4(m.matrixWorld));
        const tint = new THREE.Mesh(
          new THREE.BufferGeometry().setFromPoints(p),
          new THREE.MeshBasicMaterial({
            color,
            transparent: true,
            opacity: 0.35,
            depthTest: false,
            side: THREE.DoubleSide,
          }),
        );
        tint.renderOrder = 6;
        overlay.add(tint);
      }
    }
  }
  canvas.addEventListener('keydown', keydown);
  canvas.addEventListener('model-command', onCommand);
  canvas.addEventListener('pointerdown', pointerdown, true);
  canvas.addEventListener('pointermove', pointermove);
  canvas.addEventListener('pointerup', pointerup);
  const context = (e: Event) => e.preventDefault();
  canvas.addEventListener('contextmenu', context);
  return {
    update,
    handlesPointer: () => !useScene.getState().pickWheels && !useScene.getState().rigBrush,
    dispose() {
      cancel();
      clearOverlay();
      scene.remove(overlay);
      scene.remove(joint);
      joint.geometry.dispose();
      (joint.material as THREE.Material).dispose();
      axes.geometry.dispose();
      (axes.material as THREE.Material).dispose();
      box.remove();
      canvas.removeEventListener('keydown', keydown);
      canvas.removeEventListener('model-command', onCommand);
      canvas.removeEventListener('pointerdown', pointerdown, true);
      canvas.removeEventListener('pointermove', pointermove);
      canvas.removeEventListener('pointerup', pointerup);
      canvas.removeEventListener('contextmenu', context);
    },
  };
}

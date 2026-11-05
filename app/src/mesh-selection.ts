import * as THREE from 'three';
import type { Operation, Selection } from './bindings';
export const emptySelection = (mode: string): Selection => ({
  mode,
  vertices: [],
  edges: [],
  faces: [],
});
export function topology(g: THREE.BufferGeometry) {
  const p = g.getAttribute('position');
  const positions = Array.from({ length: p.count }, (_, i) =>
    new THREE.Vector3().fromBufferAttribute(p, i),
  );
  const indices = g.index ? Array.from(g.index.array) : positions.map((_, i) => i);
  const keys = positions.map((p) => p.toArray().join(','));
  const unique = new Map<string, number>();
  const logical = keys.map((key, i) => {
    if (!unique.has(key)) unique.set(key, i);
    return unique.get(key)!;
  });
  const edges = new Map<string, [number, number]>();
  for (let i = 0; i < indices.length; i += 3)
    for (let j = 0; j < 3; j++) {
      const a = logical[indices[i + j]],
        b = logical[indices[i + ((j + 1) % 3)]];
      const e: [number, number] = a < b ? [a, b] : [b, a];
      edges.set(e.join(','), e);
    }
  return { positions, indices, keys, logical, edges: [...edges.values()] };
}
export function selectedVertices(g: THREE.BufferGeometry, s: Selection) {
  const t = topology(g);
  const ids =
    s.mode === 'face'
      ? s.faces.flatMap((f) => t.indices.slice(f * 3, f * 3 + 3))
      : s.mode === 'edge'
        ? s.edges.flat()
        : s.vertices;
  const keys = new Set(ids.map((i) => t.keys[i]));
  return t.positions.map((_, i) => i).filter((i) => keys.has(t.keys[i]));
}
export function allSelection(g: THREE.BufferGeometry, mode: string): Selection {
  const t = topology(g);
  return {
    mode,
    vertices: [...new Set(t.logical)],
    edges: t.edges,
    faces: Array.from({ length: t.indices.length / 3 }, (_, i) => i),
  };
}
export function fromVertices(g: THREE.BufferGeometry, mode: string, ids: number[]): Selection {
  const t = topology(g),
    chosen = new Set(ids.map((i) => t.logical[i]));
  return {
    mode,
    vertices: [...chosen],
    edges: t.edges.filter((e) => e.every((i) => chosen.has(i))),
    faces: Array.from({ length: t.indices.length / 3 }, (_, i) => i).filter((f) =>
      t.indices.slice(f * 3, f * 3 + 3).every((i) => chosen.has(t.logical[i])),
    ),
  };
}
export function combine(a: Selection, b: Selection, toggle = false): Selection {
  const join = <T>(a: T[], b: T[]) => {
    const map = new Map(a.map((v) => [JSON.stringify(v), v]));
    for (const v of b) {
      const k = JSON.stringify(v);
      if (toggle && map.has(k)) map.delete(k);
      else map.set(k, v);
    }
    return [...map.values()];
  };
  return {
    mode: b.mode,
    vertices: join(a.vertices, b.vertices),
    edges: join(a.edges, b.edges),
    faces: join(a.faces, b.faces),
  };
}
/** Topology preview only. The Rust operation validates and writes the committed DS resource. */
export function previewEdit(g: THREE.BufferGeometry, s: Selection, op: Operation) {
  const t = topology(g),
    selected = selectedVertices(g, s),
    out = g.clone();
  if (op.kind === 'Transform') {
    const matrix = new THREE.Matrix4().compose(
      new THREE.Vector3(...op.translation),
      new THREE.Quaternion(...op.rotation),
      new THREE.Vector3(...op.scale),
    );
    const pivot = new THREE.Vector3(...op.pivot),
      p = out.getAttribute('position');
    for (const i of selected) {
      const v = t.positions[i].clone().sub(pivot).applyMatrix4(matrix).add(pivot);
      p.setXYZ(i, v.x, v.y, v.z);
    }
    p.needsUpdate = true;
    out.computeVertexNormals();
    out.computeBoundingSphere();
    return out;
  }
  if (op.kind === 'Delete' || op.kind === 'Merge') {
    const chosen = new Set(selected),
      faces = new Set(s.faces),
      logicalEdges = new Set(
        s.edges.map((e) =>
          e
            .map((i) => t.logical[i])
            .sort((a, b) => a - b)
            .join(','),
        ),
      );
    let indices = t.indices.filter((_, i) => {
      const f = Math.floor(i / 3),
        tri = t.indices.slice(f * 3, f * 3 + 3);
      if (op.kind === 'Merge') return true;
      if (s.mode === 'face') return !faces.has(f);
      if (s.mode === 'edge')
        return !tri.some((v, k) =>
          logicalEdges.has(
            [t.logical[v], t.logical[tri[(k + 1) % 3]]].sort((a, b) => a - b).join(','),
          ),
        );
      return !tri.some((v) => chosen.has(v));
    });
    if (op.kind === 'Merge' && selected.length) {
      const center = selected
        .reduce((p, i) => p.add(t.positions[i]), new THREE.Vector3())
        .divideScalar(selected.length);
      out.getAttribute('position').setXYZ(selected[0], center.x, center.y, center.z);
      indices = indices
        .map((i) => (chosen.has(i) ? selected[0] : i))
        .filter((_, i, a) => {
          const j = Math.floor(i / 3) * 3;
          return a[j] !== a[j + 1] && a[j] !== a[j + 2] && a[j + 1] !== a[j + 2];
        });
    }
    out.setIndex(indices);
    out.computeVertexNormals();
    out.computeBoundingSphere();
    return out;
  }
  if (op.kind !== 'Extrude' && op.kind !== 'Inset') return out;
  const faces = new Set(s.faces),
    boundary = new Map<string, { a: number; b: number; count: number }>(),
    ids = new Set<number>(),
    normal = new THREE.Vector3();
  for (let f = 0; f < t.indices.length / 3; f++) {
    if (s.mode !== 'face' || !faces.has(f)) continue;
    const tri = t.indices.slice(f * 3, f * 3 + 3);
    normal.add(
      t.positions[tri[1]]
        .clone()
        .sub(t.positions[tri[0]])
        .cross(t.positions[tri[2]].clone().sub(t.positions[tri[0]])),
    );
    for (let k = 0; k < 3; k++) {
      const a = tri[k],
        b = tri[(k + 1) % 3];
      ids.add(a);
      ids.add(b);
      const key = [t.logical[a], t.logical[b]].sort((a, b) => a - b).join(',');
      const edge = boundary.get(key) ?? { a, b, count: 0 };
      edge.count++;
      boundary.set(key, edge);
    }
  }
  if (s.mode === 'edge')
    for (const [a, b] of s.edges) {
      boundary.set([a, b].join(','), { a, b, count: 1 });
      ids.add(a);
      ids.add(b);
      for (const i of [a, b])
        normal.add(new THREE.Vector3().fromBufferAttribute(g.getAttribute('normal'), i));
    }
  normal.normalize();
  const edges = [...boundary.values()].filter((e) => e.count === 1),
    inward = new Map<number, THREE.Vector3[]>();
  for (const { a, b } of edges) {
    const n = normal.clone().cross(t.positions[b].clone().sub(t.positions[a])).normalize();
    for (const i of [a, b]) {
      const list = inward.get(t.logical[i]) ?? [];
      list.push(n);
      inward.set(t.logical[i], list);
    }
  }
  const remap = new Map<number, number>();
  const attributes = new Map<string, number[]>();
  for (const name of Object.keys(g.attributes))
    attributes.set(name, Array.from(g.getAttribute(name).array));
  for (const i of ids) {
    const j = t.positions.length + remap.size;
    remap.set(i, j);
    for (const [name, values] of attributes) {
      const a = g.getAttribute(name);
      for (let k = 0; k < a.itemSize; k++) values.push(a.array[i * a.itemSize + k]);
    }
    let delta = normal.clone().multiplyScalar(op.amount);
    if (op.kind === 'Inset') {
      const normals = inward.get(t.logical[i]);
      delta = new THREE.Vector3();
      if (normals) {
        for (const n of normals) delta.add(n);
        delta
          .normalize()
          .multiplyScalar(op.amount / Math.max(0.0001, delta.clone().normalize().dot(normals[0])));
      }
    }
    const p = attributes.get('position')!;
    for (let k = 0; k < 3; k++) p[j * 3 + k] += delta.getComponent(k);
  }
  const indices = t.indices.map((v, i) =>
    s.mode === 'face' && faces.has(Math.floor(i / 3)) ? remap.get(v)! : v,
  );
  for (const { a, b } of edges) indices.push(a, b, remap.get(b)!, a, remap.get(b)!, remap.get(a)!);
  for (const [name, values] of attributes)
    out.setAttribute(
      name,
      name === 'skinIndex'
        ? new THREE.Uint16BufferAttribute(values, g.getAttribute(name).itemSize)
        : new THREE.Float32BufferAttribute(values, g.getAttribute(name).itemSize),
    );
  out.setIndex(indices);
  out.computeVertexNormals();
  out.computeBoundingSphere();
  return out;
}

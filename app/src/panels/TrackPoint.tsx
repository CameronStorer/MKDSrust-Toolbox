import { useState } from 'react';
import type { SceneEntry } from '../bindings';
import { useStore } from '../store';
export function TrackPoint({ entry, path }: { entry: SceneEntry; path: string }) {
  const [values, setValues] = useState<number[]>(entry.section === 'CPOI' ? [...(entry.position ?? [0, 0, 0]).filter((_, i) => i !== 1), ...(entry.end ?? [0, 0, 0]).filter((_, i) => i !== 1)] : entry.position ?? [0, 0, 0]);
  const labels = entry.section === 'CPOI' ? ['Left X', 'Left Z', 'Right X', 'Right Z'] : ['X', 'Y', 'Z'];
  if (!entry.position) return null;
  return <div className="inset engine-tools"><div className="section-label">MOVE {entry.section} {entry.index}</div>{values.map((v, i) => <label key={i}>{labels[i]}<input aria-label={`Point ${labels[i]}`} type="number" step="0.1" value={v} onChange={(e) => setValues(values.map((n, j) => i === j ? Number(e.target.value) : n))} /></label>)}<button onClick={() => void useStore.getState().run({ kind: 'Wizard', action: entry.section === 'CPOI' ? { kind: 'Checkpoint', path, index: entry.index, left: [values[0], values[1]], right: [values[2], values[3]] } : { kind: 'NkmPoint', path, section: entry.section, index: entry.index, position: [values[0], values[1], values[2]] } })}>Save point</button></div>;
}

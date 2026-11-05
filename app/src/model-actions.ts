import type { ModelAction } from './bindings';
import { useScene } from './scene-view';
import { useStore } from './store';

export function runModel(actions: ModelAction[]) {
  const s = useScene.getState();
  if (s.info?.model_path?.includes('/course_model'))
    actions = actions.map((a) => {
      const units = (p: [number, number, number]): [number, number, number] => [
        p[0] / 16,
        p[1] / 16,
        p[2] / 16,
      ];
      if (a.kind === 'Transform') return { ...a, translation: units(a.translation) };
      if (a.kind === 'Edit') {
        const op = a.operation;
        return {
          ...a,
          operation:
            op.kind === 'Transform'
              ? { ...op, translation: units(op.translation), pivot: units(op.pivot) }
              : op.kind === 'Extrude' || op.kind === 'Inset'
                ? { ...op, amount: op.amount / 16 }
                : op,
        };
      }
      return a;
    });
  if (s.info?.model_path && actions.length)
    return useStore.getState().run({
      kind: 'ModelBatch',
      path: s.info.model_path,
      actions,
      linked: s.linked,
    });
  return Promise.resolve();
}
export function selectPart(part: number, add = false) {
  const s = useScene.getState();
  useScene.setState({
    part,
    parts: add
      ? s.parts.includes(part)
        ? s.parts.filter((p) => p !== part)
        : [...s.parts, part]
      : [part],
    highlight: 'none',
  });
}

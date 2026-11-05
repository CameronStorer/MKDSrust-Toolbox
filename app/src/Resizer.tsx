import { useEffect, useRef } from 'react';

/**
 * A drag handle between two sections. It sets the CSS variable `--<name>` (in px) on its parent, which the
 * layout uses for the neighbouring section's size. The size is remembered per handle; double-click resets it.
 * `invert` resizes the section after the handle instead of the one before it.
 */
export function Resizer({
  name,
  axis = 'x',
  min = 120,
  max = 1200,
  invert = false,
}: {
  name: string;
  axis?: 'x' | 'y';
  min?: number;
  max?: number;
  invert?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const key = `toolbox-size:${name}`;
  useEffect(() => {
    try {
      const saved = localStorage.getItem(key);
      if (saved) ref.current?.parentElement?.style.setProperty(`--${name}`, `${saved}px`);
    } catch {
      /* storage unavailable: use the default size */
    }
  }, [key, name]);
  const down = (e: React.PointerEvent<HTMLDivElement>) => {
    const handle = e.currentTarget;
    const parent = handle.parentElement;
    const neighbour = (
      invert ? handle.nextElementSibling : handle.previousElementSibling
    ) as HTMLElement | null;
    if (!parent || !neighbour) return;
    e.preventDefault();
    handle.setPointerCapture(e.pointerId);
    const rect = neighbour.getBoundingClientRect();
    const start = axis === 'x' ? e.clientX : e.clientY;
    const size = axis === 'x' ? rect.width : rect.height;
    let latest = size;
    const move = (m: PointerEvent) => {
      const delta = (axis === 'x' ? m.clientX : m.clientY) - start;
      latest = Math.round(Math.min(max, Math.max(min, size + (invert ? -delta : delta))));
      parent.style.setProperty(`--${name}`, `${latest}px`);
    };
    const up = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', up);
      handle.removeEventListener('pointercancel', up);
      document.body.classList.remove(`resizing-${axis}`);
      try {
        localStorage.setItem(key, String(latest));
      } catch {
        /* not remembered */
      }
    };
    document.body.classList.add(`resizing-${axis}`);
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up);
    handle.addEventListener('pointercancel', up);
  };
  const reset = () => {
    ref.current?.parentElement?.style.removeProperty(`--${name}`);
    try {
      localStorage.removeItem(key);
    } catch {
      /* nothing saved */
    }
  };
  return (
    <div
      ref={ref}
      className={`resizer resizer-${axis}`}
      role="separator"
      aria-orientation={axis === 'x' ? 'vertical' : 'horizontal'}
      title="Drag to resize · double-click to reset"
      onPointerDown={down}
      onDoubleClick={reset}
    />
  );
}

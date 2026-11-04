export type Tool =
  'pan' | 'pencil' | 'fill' | 'line' | 'rectangle' | 'eyedropper' | 'select' | 'move';
export type Point = [number, number];
export function line(a: Point, b: Point): Point[] {
  const points: Point[] = [];
  let [x, y] = a;
  const dx = Math.abs(b[0] - x),
    dy = Math.abs(b[1] - y),
    sx = x < b[0] ? 1 : -1,
    sy = y < b[1] ? 1 : -1;
  let err = dx - dy;
  for (;;) {
    points.push([x, y]);
    if (x === b[0] && y === b[1]) break;
    const e = err * 2;
    if (e > -dy) {
      err -= dy;
      x += sx;
    }
    if (e < dx) {
      err += dx;
      y += sy;
    }
  }
  return points;
}
export function stroke(
  tool: Tool,
  data: number[],
  width: number,
  height: number,
  a: Point,
  b: Point,
  color: number,
): [number, number, number][] {
  const valid = ([x, y]: Point) => x >= 0 && y >= 0 && x < width && y < height;
  let points: Point[] = [];
  if (tool === 'fill') {
    if (!valid(a)) return [];
    const target = data[a[1] * width + a[0]];
    if (target === color) return [];
    const seen = new Set<number>(),
      todo: Point[] = [a];
    while (todo.length) {
      const p = todo.pop()!;
      if (!valid(p)) continue;
      const i = p[1] * width + p[0];
      if (seen.has(i) || data[i] !== target) continue;
      seen.add(i);
      points.push(p);
      todo.push([p[0] - 1, p[1]], [p[0] + 1, p[1]], [p[0], p[1] - 1], [p[0], p[1] + 1]);
    }
  } else if (tool === 'rectangle') {
    for (let y = Math.min(a[1], b[1]); y <= Math.max(a[1], b[1]); y++)
      for (let x = Math.min(a[0], b[0]); x <= Math.max(a[0], b[0]); x++) points.push([x, y]);
  } else points = line(a, b);
  return points.filter(valid).map(([x, y]) => [x, y, color]);
}

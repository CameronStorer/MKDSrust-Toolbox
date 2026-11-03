export function rgb555(value: number): [number, number, number] {
  return [value & 31, (value >> 5) & 31, (value >> 10) & 31];
}
export function hex555(value: number): string {
  return (
    '#' +
    rgb555(value)
      .map((c) => ((c << 3) | (c >> 2)).toString(16).padStart(2, '0'))
      .join('')
  );
}
export function fromHex(hex: string): number {
  const n = parseInt(hex.replace('#', ''), 16);
  return ((n >> 19) & 31) | (((n >> 11) & 31) << 5) | (((n >> 3) & 31) << 10);
}
export function shift555(
  value: number,
  hue: number,
  saturation: number,
  brightness: number,
): number {
  const [r, g, b] = rgb555(value).map((v) => v / 31);
  const max = Math.max(r, g, b),
    min = Math.min(r, g, b),
    d = max - min;
  let h =
    d === 0 ? 0 : max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  h = (((h * 60 + hue) % 360) + 360) % 360;
  const clamp = (v: number) => Math.min(1, Math.max(0, v));
  const s = clamp((max === 0 ? 0 : d / max) + saturation / 100),
    v = clamp(max + brightness / 100);
  const c = v * s,
    x = c * (1 - Math.abs(((h / 60) % 2) - 1)),
    m = v - c;
  const [rr, gg, bb] =
    h < 60
      ? [c, x, 0]
      : h < 120
        ? [x, c, 0]
        : h < 180
          ? [0, c, x]
          : h < 240
            ? [0, x, c]
            : h < 300
              ? [x, 0, c]
              : [c, 0, x];
  return (
    Math.round((rr + m) * 31) | (Math.round((gg + m) * 31) << 5) | (Math.round((bb + m) * 31) << 10)
  );
}

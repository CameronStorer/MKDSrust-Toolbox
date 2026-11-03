import { describe, it, expect } from 'vitest';
import { fromHex, hex555, rgb555, shift555 } from './colors';
describe('DS colour editing', () => {
  it('roundtrips every representable BGR555 colour', () => {
    for (let i = 0; i < 32768; i++) expect(fromHex(hex555(i))).toBe(i);
  });
  it('preserves unchanged HSV and rotates primary colours', () => {
    expect(shift555(31, 120, 0, 0)).toBe(31 << 5);
    expect(shift555(0, 0, 0, 100)).toBe(32767);
    expect(rgb555(32767)).toEqual([31, 31, 31]);
  });
});

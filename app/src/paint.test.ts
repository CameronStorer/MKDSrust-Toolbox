import { expect, test } from 'vitest';
import { line, stroke } from './paint';
test('connected lines, bounded rectangles and index-aware flood fill', () => {
  expect(line([3, 1], [0, 1])).toEqual([
    [3, 1],
    [2, 1],
    [1, 1],
    [0, 1],
  ]);
  expect(stroke('rectangle', [], 2, 2, [-1, -1], [1, 1], 3)).toHaveLength(4);
  expect(stroke('fill', [1, 1, 2, 1, 2, 2], 3, 2, [0, 0], [0, 0], 4)).toEqual(
    expect.arrayContaining([
      [0, 0, 4],
      [1, 0, 4],
      [0, 1, 4],
    ]),
  );
  expect(stroke('fill', [1, 1, 2, 1, 2, 2], 3, 2, [0, 0], [0, 0], 4)).toHaveLength(3);
});

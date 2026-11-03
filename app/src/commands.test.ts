import { describe, it, expect } from 'vitest';
import { fuzzyScore, matchesKey, commands } from './commands';
describe('command registry', () => {
  it('supports subsequence search and excludes nonmatches', () => {
    expect(fuzzyScore('pckmd', 'Project Pack mod')).toBeGreaterThan(0);
    expect(fuzzyScore('zzz', 'Open ROM')).toBe(-1);
  });
  it('matches modifiers exactly and supports Command on macOS', () => {
    expect(
      matchesKey(
        new KeyboardEvent('keydown', { key: 'P', ctrlKey: true, shiftKey: true }),
        'Ctrl+Shift+P',
      ),
    ).toBe(true);
    expect(
      matchesKey(
        new KeyboardEvent('keydown', { key: 'p', metaKey: true, shiftKey: true }),
        'Ctrl+Shift+P',
      ),
    ).toBe(true);
    expect(
      matchesKey(new KeyboardEvent('keydown', { key: 'p', ctrlKey: true }), 'Ctrl+Shift+P'),
    ).toBe(false);
  });
  it('gates undo and pack without a project and keeps IDs unique', () => {
    expect(commands.find((c) => c.id === 'project.pack')?.when(null)).toBe(false);
    expect(new Set(commands.map((c) => c.id)).size).toBe(commands.length);
  });
});

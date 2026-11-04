export function language(path: string) {
  for (const [code, name] of [
    ['us', 'English (US)'],
    ['en', 'English (US)'],
    ['es', 'Spanish'],
    ['fr', 'French'],
    ['ge', 'German'],
    ['it', 'Italian'],
    ['ja', 'Japanese'],
  ]) {
    if (path.toLowerCase().includes(`_${code}.`) || path.toLowerCase().endsWith(`/${code}.bmg`))
      return name;
  }
  return 'Shared / language independent';
}
export function textPriority(path: string) {
  return (language(path) === 'English (US)' ? 0 : 10) + (path.endsWith('/common.bmg') ? 0 : 1);
}
export function messageGroup(path: string, index: number) {
  if (path.endsWith('common.bmg'))
    return index >= 37 && index <= 44
      ? 'Cup names'
      : index >= 45 && index <= 76
        ? 'Course names'
        : 'Common menus';
  if (path.includes('kart_select')) return 'Character & kart names';
  if (/result|secret/i.test(path)) return 'Results & unlocks';
  return (
    path
      .split('/')
      .pop()
      ?.replace(/\.bmg$/i, '')
      .replaceAll('_', ' ') ?? 'Menu'
  );
}
// Unknown codes keep their exact bytes and a neutral label: never guess semantics.
export function controlLabel(bytes: number[]) {
  return `Game control ${bytes[3] ?? ''}`;
}

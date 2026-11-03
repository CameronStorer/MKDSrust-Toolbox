import { useEffect, useRef, useState } from 'react';
import { Search, Command as CommandIcon } from 'lucide-react';
import { commands, fuzzyScore, invokeCommand } from './commands';
import { useStore } from './store';
export function CommandPalette() {
  const { snapshot, set } = useStore();
  const [query, setQuery] = useState('');
  const [index, setIndex] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const list = commands
    .filter((c) => c.when(snapshot))
    .map((c) => ({ command: c, score: fuzzyScore(query, `${c.category} ${c.title}`) }))
    .filter((c) => c.score >= 0)
    .sort((a, b) => b.score - a.score)
    .map((c) => c.command);
  useEffect(() => {
    input.current?.focus();
  }, []);
  const choose = (i: number) => {
    const c = list[i];
    if (c) {
      set({ paletteOpen: false });
      invokeCommand(c);
    }
  };
  return (
    <div
      className="modal-backdrop palette-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) set({ paletteOpen: false });
      }}
    >
      <div
        className="command-palette modal"
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
      >
        <div className="command-search">
          <Search size={17} />
          <input
            ref={input}
            placeholder="What would you like to do?"
            aria-label="Search commands"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setIndex(0);
            }}
            onKeyDown={(e) => {
              if (e.key === 'ArrowDown') {
                e.preventDefault();
                setIndex((i) => Math.min(i + 1, list.length - 1));
              }
              if (e.key === 'ArrowUp') {
                e.preventDefault();
                setIndex((i) => Math.max(i - 1, 0));
              }
              if (e.key === 'Enter') {
                e.preventDefault();
                choose(index);
              }
              if (e.key === 'Escape') set({ paletteOpen: false });
            }}
          />
          <kbd>ESC</kbd>
        </div>
        <div className="command-list" role="listbox" aria-label="Available commands">
          {list.map((c, i) => (
            <button
              className={i === index ? 'selected' : ''}
              role="option"
              aria-selected={i === index}
              key={c.id}
              onMouseEnter={() => setIndex(i)}
              onClick={() => choose(i)}
            >
              <CommandIcon size={14} />
              <span className="command-category">{c.category}</span>
              <span>{c.title}</span>
              <div className="grow" />
              <kbd>{snapshot?.settings.keybindings[c.id] ?? c.shortcut}</kbd>
            </button>
          ))}
          {!list.length && <div className="empty">No commands match “{query}”.</div>}
        </div>
        <div className="panel-foot">
          ↑ ↓ navigate <span>↵ run command</span>
        </div>
      </div>
    </div>
  );
}

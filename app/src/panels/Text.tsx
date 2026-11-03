import { useEffect, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Search, Type, Save, Code2 } from 'lucide-react';
import { select, useStore } from '../store';
import type { TextToken } from '../bindings';
function editableTokens(tokens: TextToken[]): TextToken[] {
  const result: TextToken[] = [];
  for (const token of tokens) {
    if (token.kind === 'Code' && result.at(-1)?.kind !== 'Text')
      result.push({ kind: 'Text', value: '' });
    result.push(token);
  }
  if (result.at(-1)?.kind !== 'Text') result.push({ kind: 'Text', value: '' });
  return result;
}
function meaningfulTokens(tokens: TextToken[]): TextToken[] {
  return tokens.filter((t) => t.kind !== 'Text' || t.value !== '');
}
export function MessageList() {
  const snapshot = useStore((s) => s.snapshot);
  const [query, setQuery] = useState('');
  const ref = useRef<HTMLDivElement>(null);
  const [index, setIndex] = useState(0);
  const messages = (snapshot?.preview.messages ?? [])
    .map((m, i) => ({ ...m, index: i }))
    .filter((m) => `${m.id} ${m.preview}`.toLowerCase().includes(query.toLowerCase()));
  const virtual = useVirtualizer({
    count: messages.length,
    getScrollElement: () => ref.current,
    estimateSize: () => 54,
    overscan: 8,
  });
  useEffect(() => {
    setIndex(0);
    window.dispatchEvent(new CustomEvent('message-select', { detail: 0 }));
  }, [snapshot?.selected]);
  return (
    <div className="panel messages">
      <div className="source-controls">
        <label>
          Language / file
          <select
            aria-label="BMG file"
            value={
              snapshot?.files.some((f) => f.path === snapshot.selected && f.format === 'bmg')
                ? snapshot.selected
                : ''
            }
            onChange={(e) => select(e.target.value)}
          >
            <option value="" disabled>
              Select a BMG
            </option>
            {snapshot?.files
              .filter((f) => f.format === 'bmg')
              .map((f) => (
                <option key={f.path} value={f.path}>
                  {f.path}
                </option>
              ))}
          </select>
        </label>
      </div>
      <div className="search">
        <Search size={13} />
        <input
          aria-label="Search messages"
          placeholder="Search messages or ID…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>
      <div className="tree-source">
        MESSAGES <span>{messages.length}</span>
      </div>
      <div ref={ref} className="message-scroll">
        <div style={{ height: virtual.getTotalSize(), position: 'relative' }}>
          {virtual.getVirtualItems().map((item) => {
            const m = messages[item.index];
            return (
              <button
                key={m.index}
                className={`message-row ${index === m.index ? 'selected' : ''}`}
                style={{ position: 'absolute', top: item.start, height: item.size }}
                onClick={() => {
                  setIndex(m.index);
                  window.dispatchEvent(new CustomEvent('message-select', { detail: m.index }));
                }}
              >
                <span className="mono">{String(m.id).padStart(4, '0')}</span>
                <div>{m.preview}</div>
              </button>
            );
          })}
        </div>
      </div>
      <div className="panel-foot">
        UTF-16 <span>Escape codes preserved</span>
      </div>
    </div>
  );
}
export function TextEditor() {
  const snapshot = useStore((s) => s.snapshot);
  const run = useStore((s) => s.run);
  const [index, setIndex] = useState(0);
  const [tokens, setTokens] = useState<TextToken[]>([]);
  const [insert, setInsert] = useState(false);
  const [code, setCode] = useState('1A 00 06 01 00 00');
  const [localError, setLocalError] = useState('');
  const message = snapshot?.preview.messages[index];
  const serializedTokens = JSON.stringify(message?.tokens ?? []);
  useEffect(() => {
    const listener = (event: Event) => setIndex((event as CustomEvent<number>).detail);
    window.addEventListener('message-select', listener);
    return () => window.removeEventListener('message-select', listener);
  }, []);
  useEffect(() => setIndex(0), [snapshot?.selected]);
  useEffect(() => {
    setTokens(editableTokens(JSON.parse(serializedTokens) as TextToken[]));
    setLocalError('');
  }, [serializedTokens, snapshot?.selected, index]);
  const save = () => {
    if (snapshot?.selected && snapshot.manifest && message)
      void run({ kind: 'Text', path: snapshot.selected, index, tokens: meaningfulTokens(tokens) });
  };
  const length = tokens.reduce((n, t) => n + (t.kind === 'Text' ? t.value.length : 0), 0);
  const dirty =
    JSON.stringify(meaningfulTokens(tokens)) !==
    JSON.stringify(meaningfulTokens(message?.tokens ?? []));
  return (
    <div
      className="panel text-editor"
      onKeyDown={(e) => {
        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
          e.preventDefault();
          save();
        }
      }}
    >
      <div className="toolbar">
        <Type size={14} />
        <span>MESSAGE {message ? String(message.id).padStart(4, '0') : '—'}</span>
        {dirty && <span className="edited-dot" />}
        <div className="grow" />
        <button disabled={!message || !snapshot?.manifest} onClick={() => setInsert(!insert)}>
          <Code2 size={13} />
          Insert code
        </button>
        <button className="accent" disabled={!dirty || !snapshot?.manifest} onClick={save}>
          <Save size={13} />
          Save message
        </button>
      </div>
      {message ? (
        <div className="text-editor-body">
          <div className="text-kicker">MESSAGE CONTENT</div>
          <p className="hint">Edit text between the control codes. Codes keep their exact bytes.</p>
          <div className="token-editor">
            {tokens.map((token, i) =>
              token.kind === 'Code' ? (
                <div className="code-chip" key={i} title="Immutable game control code">
                  <Code2 size={13} />
                  <span>CONTROL</span>
                  <code>
                    {token.value
                      .map((b) => b.toString(16).padStart(2, '0').toUpperCase())
                      .join(' ')}
                  </code>
                </div>
              ) : (
                <textarea
                  key={i}
                  aria-label={`Text segment ${i + 1}`}
                  value={token.value}
                  spellCheck={false}
                  disabled={!snapshot?.manifest}
                  onChange={(e) =>
                    setTokens((old) =>
                      old.map((t, n) => (n === i ? { kind: 'Text', value: e.target.value } : t)),
                    )
                  }
                />
              ),
            )}
            {tokens.length === 0 && (
              <textarea
                aria-label="Empty message"
                value=""
                onChange={(e) => setTokens([{ kind: 'Text', value: e.target.value }])}
              />
            )}
          </div>
          {insert && (
            <div className="insert-code">
              <label>
                Raw control code (hex bytes)
                <input
                  aria-label="Control code bytes"
                  className="mono"
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                />
              </label>
              <button
                onClick={() => {
                  const hex = code.replaceAll(' ', '');
                  if (!/^(?:[0-9a-fA-F]{2})+$/.test(hex)) {
                    setLocalError('Enter hexadecimal byte pairs.');
                    return;
                  }
                  const bytes = Array.from(hex.match(/../g) ?? [], (b) => parseInt(b, 16));
                  if (
                    bytes.length < 4 ||
                    bytes.length % 2 !== 0 ||
                    bytes[0] !== 26 ||
                    bytes[1] !== 0 ||
                    bytes[2] !== bytes.length
                  ) {
                    setLocalError('Code must begin 1A 00, then its total even byte length.');
                    return;
                  }
                  setTokens((old) => [
                    ...old,
                    { kind: 'Code', value: bytes },
                    { kind: 'Text', value: '' },
                  ]);
                  setInsert(false);
                  setLocalError('');
                }}
              >
                Insert at end
              </button>
              <p className="hint">
                Use a code appropriate for this message. Codes have game-specific meanings.
              </p>
            </div>
          )}
          {localError && <p className="inline-error">{localError}</p>}
          <div className="text-kicker preview-heading">READING PREVIEW</div>
          <div className="reading-preview">
            {tokens.map((t, i) =>
              t.kind === 'Text' ? (
                <span key={i}>{t.value}</span>
              ) : (
                <span key={i} className="code-placeholder">
                  ◆
                </span>
              ),
            )}
          </div>
          <p className="hint">
            Layout and wrapping depend on the game. Preview control markers are symbolic.
          </p>
        </div>
      ) : (
        <div className="empty">
          <Type size={30} />
          <p>Choose a BMG file in the message list.</p>
        </div>
      )}
      <div className="panel-foot">
        <span>
          {length} UTF-16 units · {tokens.filter((t) => t.kind === 'Code').length} control codes
        </span>
        <span>Ctrl Enter to save</span>
      </div>
    </div>
  );
}

import { useEffect, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Search, Type, Save, Code2 } from 'lucide-react';
import { draft, select, useStore } from '../store';
import { inspect } from '../backend';
import { language, textPriority, messageGroup, controlLabel } from '../friendly';
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
  const index = useStore((s) => s.textIndex);
  const setIndex = (textIndex: number) => useStore.getState().set({ textIndex });
  const [all, setAll] = useState<{ path: string; index: number; preview: string; id: number }[]>(
    [],
  );
  const [searching, setSearching] = useState(false);
  const [lang, setLang] = useState('English (US)');
  useEffect(() => {
    if (snapshot?.files.some((f) => f.path === snapshot.selected && f.format === 'bmg'))
      setLang(language(snapshot.selected));
  }, [snapshot?.selected, snapshot?.files]);
  const [group, setGroup] = useState('');
  const [across, setAcross] = useState(false);
  useEffect(() => {
    if (!across || !query.trim()) {
      setAll([]);
      return;
    }
    let alive = true;
    setSearching(true);
    const timer = setTimeout(() => {
      void Promise.all(
        (snapshot?.files ?? [])
          .filter((f) => f.format === 'bmg')
          .map(async (f) => {
            const p = await inspect(f.path);
            return p.messages.map((m, index) => ({
              path: f.path,
              index,
              id: m.id,
              preview: m.preview,
            }));
          }),
      )
        .then((result) => {
          if (alive) setAll(result.flat());
        })
        .catch((e) => useStore.getState().set({ error: String(e) }))
        .finally(() => {
          if (alive) setSearching(false);
        });
    }, 200);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [across, query, snapshot?.files]);
  const messages = (
    across && query.trim()
      ? all
      : (snapshot?.preview.messages ?? []).map((m) => ({ ...m, path: snapshot?.selected ?? '' }))
  )
    .map((m, i) => ({ ...m, index: 'index' in m ? (m.index as number) : i }))
    .filter((m) => !group || messageGroup(m.path, m.index) === group)
    .filter((m) => `${m.id} ${m.preview}`.toLowerCase().includes(query.toLowerCase()));
  const virtual = useVirtualizer({
    count: messages.length,
    getScrollElement: () => ref.current,
    estimateSize: () => 54,
    overscan: 8,
  });
  return (
    <div className="panel messages">
      <div className="source-controls">
        <label>
          Language
          <select
            aria-label="Text language"
            value={lang}
            onChange={(e) => {
              setLang(e.target.value);
              const current = snapshot?.selected.split('/').pop();
              const files =
                snapshot?.files.filter(
                  (f) => f.format === 'bmg' && language(f.path) === e.target.value,
                ) ?? [];
              const file = files.find((f) => f.name === current) ?? files[0];
              if (file) select(file.path);
            }}
          >
            {[
              'English (US)',
              'Spanish',
              'French',
              'German',
              'Italian',
              'Japanese',
              'Shared / language independent',
            ].map((l) => (
              <option key={l}>{l}</option>
            ))}
          </select>
          <select
            aria-label="BMG file"
            value={snapshot?.selected ?? ''}
            onChange={(e) => select(e.target.value)}
          >
            <option value="" disabled>
              Choose text to edit
            </option>
            {[...(snapshot?.files ?? [])]
              .filter((f) => f.format === 'bmg' && language(f.path) === lang)
              .sort((a, b) => textPriority(a.path) - textPriority(b.path))
              .map((f) => (
                <option key={f.path} value={f.path}>
                  {messageGroup(f.path, 0)} - {f.name.replace('.bmg', '')}
                </option>
              ))}
          </select>
          <select
            aria-label="Message group"
            value={group}
            onChange={(e) => setGroup(e.target.value)}
          >
            <option value="">All message groups</option>
            {[
              'Cup names',
              'Course names',
              'Common menus',
              'Character & kart names',
              'Results & unlocks',
            ].map((g) => (
              <option key={g}>{g}</option>
            ))}
          </select>
        </label>
        <label>
          <input type="checkbox" checked={across} onChange={(e) => setAcross(e.target.checked)} />
          Search all text and languages
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
        MESSAGES <span>{searching ? 'Searching...' : messages.length}</span>
      </div>
      <div ref={ref} className="message-scroll">
        <div style={{ height: virtual.getTotalSize(), position: 'relative' }}>
          {virtual.getVirtualItems().map((item) => {
            const m = messages[item.index];
            return (
              <button
                key={`${m.path}:${m.index}`}
                className={`message-row ${index === m.index ? 'selected' : ''}`}
                style={{ position: 'absolute', top: item.start, height: item.size }}
                onClick={() => {
                  if (m.path !== snapshot?.selected)
                    void useStore
                      .getState()
                      .run({ kind: 'Select', path: m.path })
                      .then(() => setIndex(m.index));
                  else setIndex(m.index);
                }}
              >
                <span className="mono">{String(m.id).padStart(4, '0')}</span>
                <div>
                  <small>
                    {messageGroup(m.path, m.index)}
                    {across ? ` - ${language(m.path)}` : ''}
                  </small>
                  <br />
                  {m.preview}
                </div>
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
  const index = useStore((s) => s.textIndex);

  const [tokens, setTokens] = useState<TextToken[]>([]);
  const [insert, setInsert] = useState(false);
  const [code, setCode] = useState('1A 00 06 01 00 00');
  const [localError, setLocalError] = useState('');
  const message = snapshot?.preview.messages[index];
  const serializedTokens = JSON.stringify(message?.tokens ?? []);
  const draftKey = `text:${snapshot?.selected}:${index}`;
  useEffect(() => {
    const pending = useStore.getState().drafts[draftKey];
    setTokens(
      editableTokens(
        pending?.kind === 'Text' ? pending.tokens : (JSON.parse(serializedTokens) as TextToken[]),
      ),
    );
    setLocalError('');
  }, [serializedTokens, snapshot?.selected, index, draftKey]);
  const save = () => {
    if (snapshot?.selected && snapshot.manifest && message)
      void run({
        kind: 'Text',
        path: snapshot.selected,
        index,
        tokens: meaningfulTokens(tokens),
      }).then(() => {
        if (!useStore.getState().error) draft(draftKey, null);
      });
  };
  const changeTokens = (next: TextToken[]) => {
    setTokens(next);
    draft(
      draftKey,
      snapshot?.selected &&
        JSON.stringify(meaningfulTokens(next)) !==
          JSON.stringify(meaningfulTokens(message?.tokens ?? []))
        ? { kind: 'Text', path: snapshot.selected, index, tokens: meaningfulTokens(next) }
        : null,
    );
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
                  <span>{controlLabel(token.value)}</span>
                  <code className={useStore.getState().advanced ? '' : 'technical-code'}>
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
                    changeTokens(
                      tokens.map((t, n) => (n === i ? { kind: 'Text', value: e.target.value } : t)),
                    )
                  }
                />
              ),
            )}
            {tokens.length === 0 && (
              <textarea
                aria-label="Empty message"
                value=""
                onChange={(e) => changeTokens([{ kind: 'Text', value: e.target.value }])}
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
                  changeTokens([
                    ...tokens,
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

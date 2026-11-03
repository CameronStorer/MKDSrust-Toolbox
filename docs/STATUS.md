# Stage 1 status — 2026-11-03

Stage 1 is implemented and the combined working tree has been reconciled. The required local commits remain
blocked: this session has read-only access to `.git`; `git add` and `git commit` fail creating `.git/index.lock`
with Permission denied. All source, documentation and screenshots therefore remain uncommitted on `main`.
Nothing was pushed, and the builder repository was not modified.

## Done

- One bounded format implementation: LZ77 with splice recompression; NARC/CARC with nested directories and
  metadata-preserving rebuilding; read-only ROM header/FNT/FAT, ARM sections, overlays and banner;
  NCLR/NCGR/NSCR/BMG readers and writers; read-only NSBTX dictionaries and all seven texture decoders.
  Duplicate builder parsers and unused format helpers were removed.
- Rust-owned projects with ROM hashes, editable source deltas, global apply/revert history, nested archive
  rebuilding, deterministic BSDIFF40/add packages, path/size/content checks and recursive Stage 1 validation.
  The CLI implements ls, extract, PNG/JSON conversion, pack and validate.
- Tauri/React shell with four active dockable workspaces, persistent layouts, themes, settings/keybindings,
  command palette, onboarding, virtualized Explorer/hex, live palette rendering and range adjustments,
  BMG text/control chips, project metadata/revert/pack and shared history. IPC types are generated from Rust;
  images and hex bytes use binary transport. OFL fonts and notices are bundled.
- Windows/Linux CI and 20 regenerated synthetic screenshots in `docs/screenshots/`. Browser fixtures come
  from the Rust generator, including messages, tile indices and actual synthetic file bytes.

## Verification

Passed locally on Windows: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, app lint/typecheck/vitest/production build, and the Tauri `cargo build`.
There are 28 executed synthetic Rust tests, 5 vitest tests and 25 passing Playwright tests (20 captures plus
5 workflows). Cargo used `.cache/cargo` offline throughout. `MKDS_ROM` was unset, so the optional retail test
skipped its body; it is implemented and keeps every ROM-derived byte in memory.

The installed builder accepted the generated synthetic manifest and patches through `--pack-mod`. PC exports
omit the optional `platforms` field, whose contract default is PC, to support older v1 parsers. A DS claim is
downgraded with a warning. Actual launcher GUI installation and Linux/macOS execution were not exercised here.

## Run

With Rust stable, Node 22 and the Tauri prerequisites installed: `cd app`, `npm ci`, `npm run tauri dev`.
The current checkout already has npm dependencies. Choose your AMCE revision 0 ROM, then create a project in
a separate folder. Edits save immediately; Overview packs `build/<id>-<version>.zip`.
From the repository root, `cargo run -p toolbox-cli -- --help` lists the headless commands.

For this cached environment, first set PowerShell `$env:CARGO_HOME="$PWD/.cache/cargo"` and
`$env:CARGO_NET_OFFLINE='true'` at the repository root. For browser checks in `app/`, set
`$env:PLAYWRIGHT_BROWSERS_PATH="$PWD/../.cache/playwright"` and `$env:MKDSTB_SCREENSHOT_PREVIEW='1'`, then run
`npm run test:e2e`. The sandbox denies the ancestor-directory reads needed by Vite's dev dependency optimizer;
the verified preview mode uses a separate ignored `.cache/screenshots-dist` and leaves the native build intact.
Normal environments use the default Vite dev capture mode. Full command examples are in `README.md`.

## Outside Stage 1 / remaining

PNG palette import/quantization is the optional stretch goal and is not implemented. New top-level files can be
added through the core API/edits folder; inserting new members into existing archives is unsupported. NSBTX and
ROMs remain read-only, history is session-local, and 3D/audio/code/Test Bench/full DS checks remain future stages.
The required local commits, including screenshot commits, still need a session with writable Git metadata.

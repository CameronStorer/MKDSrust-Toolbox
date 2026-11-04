# Toolbox status — 2026-11-03

## Stage 2 checkpoint

Done: NSBMD/GX meshes, bones/materials/skinning, NSBCA joint animation, animation header listings,
exact NKM section round-trips, KCL triangles/octree; textured/skinned GLB and collision OBJ exports.
Model and read-only Track have navigation, timeline, overlays, selection, outliner and budgets.
Test Bench discovers portable installs, installs/replaces/enables packages, launches/stops and captures logs,
queries/caches courses, supplies race subsets and restarts with Ctrl+R. The tuple type error and onboarding's
positional command references are fixed. All rule 5 checks and 25 browser tests passed at this checkpoint.

## Stage 3 checkpoint — done at a basic end-to-end level

- File/Overview Export for DS: ROM and/or BPS pickers, clean-ROM/output collision protection, verified rebuild,
  changed/in-place/moved files, sizes, warnings and Show in folder. Compatibility issues are listed from Rust
  validation; packaging retains DS claims only when checks pass. Allow PC-only features defaults off.
- New Project accepts clean ROM, modded ROM or BPS. Imported bases are stored only as `base.bps` with clean hash
  and source provenance. Working sessions apply the base in memory; later edits reopen against that base.
  Final BPS output is verified against the clean ROM. Overview lists added/removed/changed files and code/overlays.
  Code/removal bases use DS output; MKDSrust packages support filesystem deltas and report unsupported bases.
- Play targets: configured emulator plus patched ROM in the per-user cache; MKDSrust with course listing,
  character/kart ids, 1–8 racers, logs/Stop/last-run; remembered flashcart folder export. Reload restarts.
- NCGR and NSBTX pencil, fill, line, rectangle, eyedropper and select/move, palette indices/direct colour,
  zoom, pixel/tile grids and project undo/redo. Paint preserves exact indices and does not grow texture files.
- PNG import uses deterministic median cut and optional error-diffusion dithering, before/after preview,
  size/VRAM readout, and one undo step for graphics plus palette. Encodes 4/16/256-colour, a3i5, a5i3 and direct
  textures, including palette creation for direct-only texture containers; repacks pools on format changes.
  PNG export covers palette, tile, screen, texture, cell and animation-frame previews (default first frame).
- NCER compositions use companion NCGR/NCLR; NANR frame lists and playback include translation/scale/rotation.
  Recipes cover menu recolouring, item icon replacement and kart textures. All IPC types are regenerated from Rust.

## Partial / next

4×4 texture encoding, IPS import and cell-position editing remain the named stretch goals. NSBTA/NSBTP/NSBMA
list headers only. The cell viewer is a basic fixed-stage preview: external OAM affine matrices, VRAM-transfer
banks, multi-cell animations and exact NANR loop policies remain partial. PNG imports preserve existing image
dimensions; painting uses raw tiles rather than an NSCR's reused/flipped view. Converting a texture's palette
format may require updating its separate model material binding; automatic material repair is later work.

DS budgets are conservative format/model-set estimates, not live frame measurements or a hardware certification.
New top-level ROM files/archive members, full FNT reconstruction and xdelta remain outside the inherited DS
rebuilder. Its existing-file growth/move path is used here; imported bases retain their own filesystem/code.
Next: run the opt-in suite against an owned ROM and smoke-test emulator/DS/MKDSrust installs, then the model-import,
rigging, track-authoring, audio and code roadmap. No real game install or hardware was exercised in this session.

## Verification and run

Passed locally on Windows: `cargo fmt --check`, workspace Clippy with `-D warnings`, workspace tests,
app lint/typecheck/Vitest/build and the Tauri `cargo build`. Six Vitest and 28 Playwright tests pass;
synthetic screenshots cover all workspaces in both themes plus export, base changes and PNG painting/import.
`MKDS_ROM` is unset: opt-in bodies skip. New synthetic tests cover encoders, paint/import history, cells,
clean-ROM protection and imported-base-plus-edit BPS reproduction. No Nintendo data was added.

With Rust stable, Node 22 and Tauri prerequisites: in `app/`, `npm ci`, then `npm run tauri dev`.
Open an AMCE revision 0 dump outside the project; create/open a project. Double-click models/courses/cells in
Explorer. Texture & 2D offers paint/import/recipes. Overview exports for DS. Configure emulator/install/flashcart
paths in Settings and choose the Play target in Test Bench. MKDSrust course discovery uses Refresh courses;
engine class stays disabled because the game does not support it. CLI: `cargo run -p toolbox-cli -- --help`.

Cached PowerShell from repository root: `$env:CARGO_HOME="$PWD/.cache/cargo"`;
`$env:CARGO_NET_OFFLINE='true'`. Browser tests from `app/`:
`$env:PLAYWRIGHT_BROWSERS_PATH="$PWD/../.cache/playwright"`; `$env:MKDSTB_SCREENSHOT_PREVIEW='1'`;
`npm run test:e2e`. Regenerate synthetic assets with `cargo run -p toolbox-core --example demo`.
For local ROM checks, set `MKDS_ROM` and run `cargo test -p toolbox-core --test retail_opt_in --test stage3`;
exports use disposable temporary directories outside the repository.

Changes are uncommitted. Nothing was pushed; Git metadata and the builder repository were not modified.

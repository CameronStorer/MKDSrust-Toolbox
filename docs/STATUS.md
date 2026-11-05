# Toolbox status - 2026-11-05

**Real hardware first:** every feature below produces a plain ROM hack (patched ROM / BPS) for a DS, flashcart or
emulator. MKDSrust is an optional play target. Not yet confirmed on a real DS or in melonDS by a human tester.

2026-11-05:
- Model view: NSBMD texture matrices (scale/rotate/translate) are now applied, so track and object textures tile
  as on the DS. 358 retail materials use one. They are kept when a model is written back.
- Top bar: a **Simple | Studio** switch replaces the hidden Advanced button and is remembered between sessions.
- Model view: the camera stays where the user left it after an edit.

Stage 6 A-H now have a first breadth-first usability pass. Stages 1-5 and the
expert editors remain available. Changes are uncommitted.

## What a beginner can do

1. Open the external USA ROM and create/open a project. Home asks **What do you
   want to make?** Seven cards open character, kart, track, music, image, text
   and cup workflows. Home also has recent items/projects and Export for DS.
2. Use **Game browser** to search friendly names or paths. Characters distinguish
   race models (P_), far-away opponents (E_), menu models and named clips.
   Tracks use the loaded cup/course names. Items stay in tabs; Back, Forward
   and Recently opened retain navigation. Model/image thumbnails are generated
   locally; course model thumbnails use a top view.
3. Open **Text**: English (US) is selected first, with separate language/file
   controls, message groups and optional search across every language/file.
   Existing control bytes stay in chips. Drafts survive navigation; **Save**,
   **Ctrl+S** (including inside text fields), or Save message writes them.
4. Open **Image gallery**: known stems, localized/shared archive pairs and sprite
   suffixes automatically choose palettes/screens. Swap a palette colour, see
   its other users, paint individual tiles, or Export PNG / Import PNG with a
   before/after conversion preview. Embedded item textures support painting,
   PNG replacement and undoable colour swapping too. Manual pairing is in Details.
5. Open **Audio**, choose a course theme/menu tune or a voice/effect group, then
   Play preview. The desktop CSP now permits blob audio. Playback uses the
   linked bank, renders through the first loop (five-minute safety bound), and
   has native seeking/duration, Stop and visible errors. Replace this song/sound
   opens its existing import controls. Conditional SSEQ events now preview.
6. Use the model cards with GLB/glTF/OBJ. Exact vertex welding uses hashing;
   quadric reduction batches independent local collapses. **31,600 -> 499
   triangles takes about 2 seconds in debug**. A generated 31,754-triangle
   humanoid with 220 bones, matrix nodes and mismatched rest/bind poses runs
   through character, kart and track wizards in about **1.5-1.7 seconds each**.
   Tracks auto-reduce to 1,000 triangles, preserve small disconnected surfaces,
   generate visible fallback textures and generate nonblank Map2D art.
7. Save As copies authored project folders/patches into an empty folder while
   retaining the external ROM reference. Closing warns about drafts or changes
   without a current export. **Advanced** exposes the raw Explorer, original
   docked workspaces, numeric controls and every previously available tool.

## Remaining limits

This is not an exhaustive asset catalog or related-assets page: some labels are
structural fallbacks, ambiguous image pairs stay in Advanced, and some per-character
icon/voice associations still need mapping. The new follow-up STAGE6B covers this.
Text's reading preview uses a system font: NFTR rendering and named meanings for
all control codes remain. Audio is a bounded basic software mixer, not an exact
hardware envelope/effects emulator; unsupported commands produce explicit errors.
Screen/cell repainting still uses the individual tile sheet. FBX/DAE still need
conversion, and arbitrary animation retargeting remains limited.

The Stage 5 root-grid KCL, current donor voices, fallback textures/minimap and
current exports still need maintainer boot tests. No new DS hardware/emulator
verification is claimed. Existing KCL size guards and retail member budgets remain.

## Verification and run

Passed: cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings;
cargo test --workspace (**86 tests with MKDS_ROM set**); app lint, typecheck,
6 Vitest tests, production build, Tauri cargo build; **61 Playwright tests**.
Home/Game browser/gallery and existing workspaces have both-theme synthetic
screenshots in docs/screenshots; stage6-* captures include Text, Swap a colour,
Audio and a wizard. No ROM-derived fixtures/screenshots were added. Owned-ROM
checks read in place and the new checks keep previews/rebuilt data in memory.

PowerShell from root: set CARGO_HOME to <repo>/.cache/cargo and CARGO_NET_OFFLINE
to true. Set MKDS_ROM to the external path in docs/STAGE4.md for retail checks.
In app/, run npm run tauri dev. Open the ROM/project, choose a Home card, Save,
then Export for DS outside the repository. Regenerate synthetic demos with
cargo run -p toolbox-core --example demo. For npm run test:e2e, set
PLAYWRIGHT_BROWSERS_PATH=<repo>/.cache/playwright and MKDSTB_SCREENSHOT_PREVIEW=1.
The preview-server setting avoids this sandbox's Vite dev optimizer access error.

No Git metadata/remotes or native-builder sources were changed; nothing was pushed.

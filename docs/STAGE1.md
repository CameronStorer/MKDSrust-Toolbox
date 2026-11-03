# Stage 1: Foundation (task for the implementing agent)

Goal: a real, usable app skeleton that already does one complete modding loop: open your ROM, browse it, recolour a
palette or edit a text message, and pack a mod package that the MKDSrust launcher installs.

## A. Repo scaffold
- A Cargo workspace (`crates/nitro-formats`, `crates/mkds-data`, `crates/toolbox-core`, `crates/toolbox-cli`,
  `app/src-tauri`).
- `app/`: Tauri 2 + React 18 + TypeScript (strict) + Vite, with ESLint, Prettier and vitest.
- GitHub Actions CI (`.github/workflows/ci.yml`) on windows-latest and ubuntu-latest, running everything in AGENTS.md
  rule 5. Linux needs the Tauri system dependencies.

## B. `nitro-formats` (pure Rust, no UI deps, `thiserror` errors with byte offsets, never panics on bad input)
Each format gets `parse(&[u8]) -> Result<T>` and `write(&T) -> Vec<u8>`, plus round-trip tests on synthetic fixtures.
- **LZ77** type 0x10: compress, decompress, and `recompress(original_compressed, new_plain)` with the splice approach
  (port from mkds-native-builder `dsfiles.rs`), so small edits give small patches.
- **NARC**: FATB/FNTB/FIMG, including nested directories. CARC = LZ77(NARC).
- **ROM**: header, FNT/FAT, file-by-path lookup, ARM9/ARM7/overlay table, banner (icon + titles).
  Read-only in stage 1.
- **NCLR**: 4bpp/8bpp, extended palettes; BGR555 ↔ RGBA8 helpers.
- **NCGR**: 4bpp/8bpp, tiled and bitmap mapping. **NSCR**: tilemap with flips and palette index.
- **BMG**: INF1/DAT1 (+ MID1 if present); UTF-16 text with escape codes kept as structured tokens, so the round trip
  is exact.
- **NSBTX**: parse texture/palette dictionaries and decode every texture format to RGBA (a3i5, 4-colour, 16-colour,
  256-colour, 4x4 compressed, a5i3, direct). Read-only in stage 1.
- A format detector by magic + extension (see PLUGIN_FORMAT.md "Allowed file types").

## C. `toolbox-core`
- **Project:** create/open a project folder with `project.toml`:
  - manifest fields: id, name, version, author, type, description, platforms;
  - the target ROM's SHA-256 and game code `AMCE`;
  - `edits/` mirroring ROM paths, where an edit of a file inside a CARC/NARC is stored under the archive path plus the
    inner path.
- **ROM session:** open the user's `.nds` in place (memory-mapped or read once), verify the game code and warn if it
  isn't AMCE, and build a virtual file tree including the contents of archives.
- **Command/undo system:** every edit is a command with apply/revert. There is one project-wide history (undo, redo,
  list).
- **Pack:** build a PLUGIN_FORMAT v1 zip:
  - for each edited ROM file, rebuild it (re-pack the inner file into the NARC, recompress the CARC with splice);
  - bsdiff it against the retail file (`qbsdiff`) as a `patch` entry; new files become `add` entries;
  - write `manifest.json`;
  - enforce the size/path limits and the rule "patch < 60% of target unless target < 64 KiB" (warn);
  - never write retail bytes into the zip, and assert this in a test.
- **Validation framework:** a `Check` trait producing `Issue { severity, location, message, why, fix: Option<FixId> }`.
  Stage 1 checks: palette colour count, texture dimensions are powers of two ≤1024, BMG message length.
  This feeds the DS-compatibility badge later.

## D. `toolbox-cli` (`mkdstb`)
`mkdstb ls <rom> [path]`, `mkdstb extract <rom> <path> -o <out>`, `mkdstb convert <file> --to png|json`,
`mkdstb pack <project>`, `mkdstb validate <project>`.

## E. App (the main deliverable is the feel: pro, dense, Blender/VS Code-like)
- **Shell:**
  - top bar with menus, workspace tabs (Overview, Explorer, Texture & 2D, Text, plus disabled placeholder tabs Track,
    Model, Rig & Animate, Audio, Code, Test Bench showing "coming in stage N"), a disabled Play button, and the
    DS-compatibility badge (shows "Unknown" for now);
  - status bar.
- **Docking:** `dockview`. Each workspace has a default layout; layouts persist per workspace and can be reset.
- **Command palette** (Ctrl+Shift+P and F3), fuzzy search over a central command registry: id, title, category,
  default shortcut, `when` condition. A keybinding editor in Settings.
- **Themes:** dark (default) and light, using CSS variables. Inter for UI, JetBrains Mono for numbers/hex (bundled OFL
  fonts with their license). Compact 24–28 px rows, 12–13 px text, one accent colour, no gradients.
- **Settings:** ROM path, MKDSrust install path (auto-detect the portable launcher folder), theme, viewport navigation
  style (stored for later).
- **First run:** pick the ROM, then create or open a project.
- **Explorer workspace:**
  - virtualized file tree (ROM + nested archives) with icons per type and an "edited" marker;
  - preview panel (image for 2D/texture formats, text for BMG, otherwise "no preview");
  - hex view (virtualized, offset/hex/ASCII, follows the selection);
  - Properties panel (size, type, offsets, compression);
  - context menu: Extract…, Export as PNG/JSON, Edit in workspace.
- **Texture & 2D workspace:**
  - canvas view of NCGR+NCLR(+NSCR) or an NSBTX texture, with zoom/pan, pixel grid, and palette-swap selector;
  - palette panel: 16×N swatches; click to edit (colour picker in BGR555 with a live RGB/hex readout); select a range
    and apply a hue/saturation/brightness shift;
  - edits go through the undo system, and the preview updates live;
  - Import PNG for a palette (quantize to the palette size) as a stretch goal.
- **Text workspace:** message list (id, preview) with search, and an editor with escape codes shown as non-editable
  chips plus an insert-code menu. Language/file selector across every BMG in the ROM.
- **Overview workspace:** project info (editable manifest fields), list of edited files with a revert action, and a
  **Pack mod** button that shows the result (path, size, any warnings).
- **History panel** available in every workspace.
- **Screenshots:** a Playwright script that launches the Vite dev UI with a mocked backend (synthetic data) and
  captures every workspace in both themes into `docs/screenshots/`.

## F. Definition of done
- Everything in AGENTS.md rule 5 passes.
- Round-trip tests for every format above pass on synthetic data.
- A `MKDS_ROM`-gated integration test (skipped without the env var) opens the ROM, lists files, round-trips every NARC
  and BMG and every NCLR it finds, edits one palette colour, packs, and checks that applying the patch to the retail
  file reproduces the edit.
- `docs/STATUS.md` written. Screenshots committed (synthetic data only).

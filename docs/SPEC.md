# MKDSrust Toolbox: product and architecture spec

Status: v0.1 draft, 2026-11-03. Owner: CameronStorer. License: GPL-3.0-or-later.

## 1. Vision
The "Blender of Mario Kart DS modding." A single professional tool that can change any part of the game:
- tracks, karts, characters, textures, 2D UI, text, audio, effects and game code;
- tested instantly in the player's own MKDSrust install.

It's a pro tool: it may take time to learn, but it lets you do anything. It still has to be easy to navigate, with
discoverable commands, consistent panels and no hidden modes.

Hard rules (non-negotiable):
1. **No Nintendo material in this repo, its releases or its CI.** Nothing copyrighted: no ROM, file, texture, model,
   sound, text or code from the game or the Nintendo SDK. Tests use synthetic data. Tests against a real ROM are
   local-only and opt-in (the env var `MKDS_ROM` points to the user's own dump) and are skipped otherwise.
2. The Toolbox reads the user's own ROM in place. Extracted data goes to a per-user cache outside the project folder,
   and never into a mod package.
3. Mods are exported only in the MKDSrust package format (`docs/PLUGIN_FORMAT.md`). That means `patch` (bsdiff against
   the retail file) or `add` (the author's own new file). Retail bytes never end up in a package.
4. No proprietary SDK tools (g3dcvtr, seqconv and so on) are required or bundled. We write DS formats ourselves.
5. Third-party code is used only under a GPL-3-compatible license, credited in `CREDITS.md`. Ideas, layouts and UX
   conventions from other tools are welcome (see `docs/PRIOR_ART.md`); unlicensed code is not.

## 2. Users
- **Players who make a small mod:** recolour a kart, change some text, swap music. They need guided flows and good
  defaults.
- **Experienced modders** used to Blender, MKTB and NitroPaint. They need speed, keyboard shortcuts, precision and
  batch tools.
- **Code modders** who write Lua gameplay scripts and Rust source patches against MKDSrust's readable source.

## 3. UI model (Blender-like, but easy to navigate)
- **Top bar:** File/Edit/View/Project menus, **workspace tabs**, the **Play** button, and the **DS-compatibility
  badge** (see 3.2).
- **Workspaces** (tabs, like Blender's): each one is a saved layout of dockable panels, built for one job. Users can
  rearrange, save and reset layouts.
  1. **Overview:** the project dashboard, recent files, what has changed compared with the ROM, build status.
  2. **Explorer:** the ROM/project file tree (with nested NARC/CARC), hex view, previews for every format, and
     "Edit in…" to jump to the right workspace.
  3. **Track:** 3D/2D scene of a course, with object/path/checkpoint/route editing and toggleable overlays:
     - collision (KCL, coloured by type);
     - CPU routes, checkpoints, respawns, item boxes, objects/NPCs, cameras;
     - per-layer visibility and lock.

     Also a polygon/texture budget readout and validation. Same scope as MKTB's course editor.
  4. **Model:** import any model (glTF/GLB, FBX, OBJ, DAE), edit the mesh (select verts/edges/faces, delete, split,
     decimate, mirror, transform) and fix materials. Includes a polygon budget and "fit to kart/driver" helpers, such
     as cutting the legs off a seated driver.
  5. **Rig & Animate:**
     - auto-fit an imported model onto the game's driver/kart skeleton;
     - weight painting;
     - retarget and preview the game's animations (steer, drift, trick, hit, win/lose) on the new model;
     - keyframe editor and timeline.
  6. **Texture & 2D:** palettes, tiles, screens, cells and textures, with painting, colour reduction, import/export as
     PNG, and conversion to every DS texture format.
  7. **Text:** BMG message editing for all languages, with special codes shown as chips.
  8. **Audio:** the SDAT tree; MIDI ↔ SSEQ, WAV ↔ SWAV/STRM, instrument banks, and a player.
  9. **Code:** a Monaco-based IDE for Lua scripts and Rust source patches. It has the game's readable source as a
     read-only reference with go-to-definition, plus a diff view and build errors linked to their lines.
  10. **Test Bench:** launch, configure and watch a race in the user's MKDSrust install:
      - pick track, kart, character, CC and number of CPUs;
      - hot-reload changed assets;
      - logs, frame timing, and a live DS budget meter.
- **Command palette** (Ctrl+Shift+P / F3): every action is searchable, shows its shortcut, and can be rebound.
- **Properties panel** (right side, as in Blender): context-sensitive fields for the current selection, with tooltips
  explaining each DS-specific field.
- **Outliner** (left): the project/scene tree for the current workspace.
- **Status bar:** a hint for the current tool, background jobs, validation counts, and the budget meter.
- **Undo/redo across the whole project** with a history panel. Every edit is a command object (see MKTB's `Actions`
  library for the pattern).
- **Familiar controls:** viewport navigation can follow Blender (default), Maya or MKTB styles. NitroPaint-like
  shortcuts in the Texture workspace.
- **Onboarding:**
  - the first launch asks for the ROM and MKDSrust install;
  - each workspace has a "?" tour;
  - recipes (guided flows) such as "Replace a kart model", "Recolour a character", "New music track". These later
    become the modding guide on docs.mkdsrust.com.
- **Look:** dark (default) and light themes, a dense pro layout, one accent colour, and no decorative gradients. It
  should look like Blender, VS Code or Figma, not a web page.

### 3.2 Real hardware compatibility
The **DS-compatibility badge** sits in the top bar and is always visible. It shows one of three states:
- **DS OK:** the project stays within real DS limits.
- **PC only:** with a reason count.
- **Unknown:** not checked yet.

Clicking it opens the **Compatibility panel**:
- each problem, with where it is (jump to file or object), why it matters on hardware, and how to fix it;
- a **Fix** button on each problem that can be fixed automatically: reduce colours, downscale or convert textures,
  decimate to a polygon budget, split meshes over the matrix-stack limit, cut text to the box size;
- **Make DS-compatible**, which runs every safe automatic fix as one undoable step and shows a before/after summary.

The checks are the same rules the site uses (`PLUGIN_FORMAT.md` "Platform compatibility"), plus format-level limits:
- polygons and vertices per frame;
- texture VRAM (512 KiB) and palette VRAM (96 KiB);
- texture sizes and formats;
- matrix stack;
- SSEQ track/channel limits;
- ROM size.

Its result fills the manifest's `platforms` field.

### 3.3 Export for real hardware
A project must also run on a real DS (flashcart) or a DS emulator, not only in MKDSrust. **Export for DS**:
- applies the project, plus optionally other enabled DS-compatible mods in load order, to the user's own ROM;
- rebuilds the ROM file system: FNT/FAT, file data alignment, overlay and banner offsets, header CRC;
- writes either a patched copy of the user's ROM, which stays on their machine and is never uploaded or shared, or a
  shareable patch file (BPS or xdelta, both supported) against the clean retail ROM;
- runs the DS-compatibility checks (3.2) first, and refuses or warns on PC-only content;
- verifies the result: it re-reads the rebuilt ROM, checks every changed file round-trips, and the header/CRC are
  valid. Optionally it boots it in MKDSrust as a smoke test.

The CLI gets the same as `mkdstb export-ds <project> --rom <rom> (--out patched.nds | --patch out.bps)`.
Packages on mkdsrust.com marked "DS compatible" can be exported the same way from the launcher.

## 4. Architecture
```
MKDSrust-Toolbox/
  crates/
    nitro-formats/     pure Rust, no UI: LZ77, NARC/CARC, ROM (header, FNT/FAT, overlays), NCLR/NCGR/NSCR/NCER/NANR,
                       NSBTX/NSBMD/NSBCA/NSBTA/NSBTP, BMG, NKM, KCL, SDAT family. Each: parse + write + round-trip tests.
    mkds-data/         MKDS-specific meaning: course/kart/character tables, NKM object IDs, file roles, DS budgets.
    toolbox-core/      project model, command/undo system, validation (DS checks), packaging (calls the
                       mkds-native-builder `mod pack` contract or reimplements it identically), import/export (glTF etc).
    toolbox-cli/       `mkdstb`: headless versions of every core feature (convert, validate, pack). Used by CI and power users.
  app/                 Tauri 2 shell (src-tauri/) + React + TypeScript + Vite UI.
    src/workspaces/    one folder per workspace; src/panels/ shared panels; src/viewport/ three.js; src/commands/.
  docs/                this spec, PRIOR_ART.md, PLUGIN_FORMAT.md, ARCHITECTURE notes, recipes.
```
- **Frontend:**
  - React + TypeScript (strict) + Vite;
  - dockable panels with `dockview` (MIT);
  - three.js for 3D viewports, with gizmos via `TransformControls` (or our own);
  - Monaco for code and the hex/text views;
  - zustand for UI state.

  Project state of record lives in Rust; the UI holds views of it.
- **Rust ↔ UI:** Tauri commands and events with typed bindings generated by `specta`/`tauri-specta`, so there are no
  hand-written duplicate types. Large binary data (textures, meshes) goes through Tauri's binary IPC or a custom
  `asset://` protocol, not JSON.
- **Project format:** a folder. `project.toml` holds the manifest fields, target ROM hash and settings. `edits/`
  mirrors ROM paths for edited and new files in editable source form (glTF, PNG, JSON, MIDI, Lua). `build/` holds
  generated outputs and is ignored by git. The project is git-friendly, and builds are reproducible from the
  project plus the user's ROM.
- **Interop with MKDSrust:** the Test Bench finds the MKDSrust launcher's portable install (`data\`, `games\`), builds
  the mod into `games/<id>/mods/<id>`, enables it, and starts the game with race-setup arguments. The game-side
  hot-reload and race-setup hooks are added in mkds-native-builder.
- **Export, everything out:** glTF 2.0/GLB (models, rigs, animations), OBJ (collision), PNG, MIDI, WAV and JSON (NKM).
  Import back from all of them.
- **Platforms:** Windows first. Linux and macOS builds should work from the start, so don't use Windows-only APIs.

## 5. Stages (each one ships)
1. **Foundation:**
   - app shell, workspaces, docking, command palette, themes, settings, undo framework;
   - open the ROM; Explorer with nested archives, hex view and format detection;
   - palette and texture viewing (NCLR/NCGR/NSCR/NSBTX), BMG viewing;
   - `nitro-formats` with LZ77/NARC/ROM/NCLR/NCGR/NSCR/BMG (parse and write);
   - edit a palette and a BMG, then pack a mod that the launcher installs.
2. **3D viewing:** NSBMD/NSBTX rendering, the Track workspace read-only (NKM + KCL overlays, budget readout), and model
   export to glTF.
3. **Test Bench:** Play button, race-setup arguments, hot reload of 2D/text, logs.
4. **Model pipeline:** glTF/FBX/OBJ import, mesh editing, decimation, NSBMD/NSBTX writer, DS checks, the
   Compatibility panel and "Make DS-compatible".
5. **Rig & Animate:** skeleton fitting, weights, retargeting, NSBCA writer.
6. **Track editing:** NKM editing (MKTB parity), KCL generation, new courses.
7. **Code:** Lua scripting API docs and the IDE, Rust source patches against the readable source.
8. **Audio:** SDAT editing, MIDI ↔ SSEQ, WAV ↔ SWAV/STRM.
9. **Effects and 2D UI layouts.**
10. **Export for real hardware** (3.3). It can come earlier: it only needs the ROM rebuild and the patch writers.

## 6. Quality bar
- `cargo test`, `cargo clippy -D warnings`, `npm run lint`, `npm run typecheck` and `npm test` (vitest) all pass in CI
  (GitHub Actions, Windows + Linux).
- Every format has write(parse(x)) == x tests on synthetic fixtures, plus fuzzing (cargo-fuzz) for parsers in later
  stages.
- Parsers never panic on bad input; they return errors with offsets.
- UI work is checked with Playwright screenshots of every workspace in both themes.

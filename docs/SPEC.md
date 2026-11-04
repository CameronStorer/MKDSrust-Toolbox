# MKDSrust Toolbox: product and architecture spec

Status: v0.2, 2026-11-03 (**real hardware first**). Owner: CameronStorer. License: GPL-3.0-or-later.

## 1. Vision
**A real-hardware ROM-hacking studio for Mario Kart DS**: the "Blender of MKDS modding". Everything it makes is a ROM
hack of the original game, playable on a DS, a flashcart or an emulator. One tool covers:
- tracks, karts, characters, textures, 2D UI, text, audio, effects and game code;
- tested instantly in an emulator, on hardware, or in MKDSrust.

**MKDSrust is an option, not the target.** It turns any ROM hack the Toolbox makes into a PC game, so DS-compatible
means PC-compatible. PC-only features (Lua scripts, HD, files over DS limits) are hidden behind a setting ("Allow
PC-only features", off by default) and are backlog work.

**Goal: even a beginner can make a ROM hack.** The acceptance scenario (section 7) is the definition of done.

It's a pro tool: it may take time to learn, but it lets you do anything. It still has to be easy to navigate, with
discoverable commands, consistent panels and no hidden modes.

Hard rules (non-negotiable):
1. **No Nintendo material in this repo, its releases or its CI.** Nothing copyrighted: no ROM, file, texture, model,
   sound, text or code from the game or the Nintendo SDK. Tests use synthetic data. Tests against a real ROM are
   local-only and opt-in (the env var `MKDS_ROM` points to the user's own dump) and are skipped otherwise.
2. The Toolbox reads the user's own ROM in place. Extracted data goes to a per-user cache outside the project folder,
   and never into a mod package.
3. What leaves the user's machine never contains retail bytes: BPS/xdelta patches against the clean ROM, MKDSrust
   packages (`docs/PLUGIN_FORMAT.md`: per-file `patch`/`add`), and the author's own files. A patched ROM is written
   only for the user's own use.
4. No proprietary SDK tools (g3dcvtr, seqconv and so on) are required or bundled. We write DS formats ourselves.
5. Third-party code is used only under a GPL-3-compatible license, credited in `CREDITS.md`. Ideas, layouts and UX
   conventions from other tools are welcome (see `docs/PRIOR_ART.md`); unlicensed code is not.

## 2. Users
- **Beginners** who want their own kart, character, track, music or Grand Prix on a real DS. They need guided flows
  ("recipes"), automatic conversion to DS limits, and Fix buttons instead of error codes.
- **Experienced ROM hackers** used to Blender, MKTB and NitroPaint. They need speed, precision, batch tools, and
  interop with DS-format files from other tools.
- **Code modders** at four levels:
  1. tweak panels (no code);
  2. hack templates (tick boxes);
  3. MKDS Script, a simple Lua-like language compiled to DS code;
  4. C/ARM assembly against the generated SDK headers.

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

## 4. Pipeline: what goes in, how you work, what comes out
**In** (all optional except the first):
- **The user's clean ROM.** USA (AMCE), identified by hash. Read in place, never modified. Extracted data is cached
  per user, outside projects.
- **An existing ROM hack** (.nds), or a **patch** (BPS/xdelta/IPS) applied to the clean ROM. It's compared with the
  clean ROM, and the changes become the project's base layer: changed files, plus code regions shown as "changed
  code".
- **MKDSrust packages:** their file changes become layers.
- **Open-format assets:** glTF/FBX/OBJ/DAE, PNG, WAV/MIDI, text tables. Converted at build.
- **DS-format files from other tools** (.nclr, .ncgr, .nsbmd, .nkm, .kcl, .sdat...): used as-is.

**Working: copies, never the originals.** A project is a git-friendly folder:
```
project.toml   name, version, base (clean ROM hash + imported hack/patch hash), settings
sources/       the author's editable sources (.glb, .png, track .json, text, music)
overrides/     DS-format files used as-is
code/          C / ARM asm / MKDS Script
build/         generated, disposable
```
- A build is a recipe: clean ROM + base layer + project changes, converted, packed into archives, then the ROM is
  rebuilt.
- Every asset can be exported to an open format and re-imported at any time.
- Watched files (saving in Blender) are picked up live.
- Projects store only the author's changes, so they're shareable without game data.
- Assets taken from other people's hacks keep a provenance note. Publishing warns: credit or ask the author.

**Out:**
- **Patched ROM** (.nds), for the user's own DS/flashcart/emulator.
- **BPS patch** (default; xdelta optional), for sharing. It checks the source ROM, so a wrong ROM is refused.
- **MKDSrust package**, for the launcher and the site.
- **Open formats:** glTF, PNG, OBJ, MIDI, WAV.

**ROM rebuild** (ndstool equivalent): NitroFS FNT/FAT, files may grow or be added, overlay table and binaries, banner,
header CRC, padding and alignment. Verified by re-reading and round-tripping every file.

**Combining:** file layers merge in load order, with a warning when two layers touch the same file. A finished
whole-ROM BPS isn't mergeable (standard for ROM hacks), so combine at project level.

**Code hacks:**
- Source in `code/`, compiled by a bundled ARM compiler (ARMv5TE/Thumb) against the generated **symbol map and
  headers**: function addresses, structs and globals from the readable-source research (USA ROM).
- Hooks are declared by name: `MKDS_HOOK_AFTER(item_roulette_select)`; before, replace, call-site and constant patches
  also exist.
- At build, the code is placed in a code file in the ROM, loaded at boot by a small loader patched into the game's
  start-up (arena allocation, as RuntimeCodeModules does). Hooks become branch instructions.
- Two mods hooking the same place are reported as a conflict.
- Imported hacks keep their code working. Editing it needs its source.

**Limits** (checked continuously):
- about 4 MB main RAM;
- 512 KB texture VRAM per scene;
- about 2,048 polygons per frame;
- 16 sound channels;
- region USA only for now (each region needs its own symbol map);
- online play (Wiimmfi) out of scope.

## 5. Architecture
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

## 6. Roadmap (real hardware first)
Done:
- **Stage 1:** shell, workspaces, ROM explorer, palette and text editing, packing.
- **Stage 2+3:** 3D viewing, the read-only Track workspace, glTF/OBJ export, Test Bench. In progress (`stage2-wip`).

Next, in order:
1. **Foundation for DS output:**
   - ROM writer (rebuild);
   - Export for DS: patched ROM, BPS, xdelta;
   - open a ROM hack or patch as a project base (diff against the clean ROM);
   - the project folder layout above;
   - Play targets: emulator (configurable path, e.g. melonDS), MKDSrust (`--race`), and export to flashcart.
2. **Textures & 2D (NitroPaint level):** pixel and tile painting, PNG import with colour reduction to the palette size,
   every texture format, cells and animations, item icons, menu graphics.
3. **Karts & characters:**
   - import any model (unrigged too) to NSBMD;
   - auto-fit to the kart and driver skeletons, auto-rig with weights;
   - trim and decimate to DS budgets;
   - textures to DS formats;
   - menu and select-screen assets.
4. **Tracks (MKTB level, plus model-to-track):**
   - import any map model;
   - paint collision types and generate KCL;
   - generate and edit start, checkpoints, CPU and item routes, respawns and cameras;
   - item boxes and objects;
   - minimap;
   - validation with Fix buttons.
5. **Audio:** SDAT editing, MIDI → SSEQ (with instrument banks), WAV → SWAV/STRM, sound effect replacement, and
   assigning music to tracks.
6. **Text & Grand Prix:** all BMG messages with a preview, cups (names and track order), track names and icons.
7. **Animations & effects:** kart and driver animations (NSBCA), texture animations, particles (SPL).
8. **Tweak panels and hack templates:** kart/character stats, item odds per place, CPU difficulty, speeds and laps;
   templates such as "200cc", "no blue shells", "item rain".
9. **Code:**
   - SDK headers and symbol map generated from the readable-source research;
   - the C/ASM injector;
   - MKDS Script;
   - **extra track slots** (CTGP-style: add tracks instead of replacing them).

Backlog (PC): Lua scripting (already in MKDSrust, behind the setting), HD/widescreen, a drop-in mod file loader.
After the Toolbox is done: site and launcher work (file types, DS-only vs PC-only labels on the site).

## 7. Acceptance scenario (definition of done)
A beginner, with the Toolbox alone and their own ROM, can:
1. import their own **character model, unrigged**, and race as it;
2. import their own **kart model, unrigged**, and drive it;
3. turn a **random map model** into a playable track (collision, routes, CPU racers, item boxes);
4. add a **song** to that track, and replace some **sound effects**;
5. replace **item textures**;
6. change **text** (menus, names);
7. make their own **Grand Prix cup** with chosen tracks;
8. export a BPS and a patched ROM that **boot and play on a real DS** (and in an emulator, and in MKDSrust).

## 8. Quality bar
- `cargo test`, `cargo clippy -D warnings`, `npm run lint`, `npm run typecheck` and `npm test` (vitest) all pass in CI
  (GitHub Actions, Windows + Linux).
- Every format has write(parse(x)) == x tests on synthetic fixtures, plus fuzzing (cargo-fuzz) for parsers in later
  stages.
- Parsers never panic on bad input; they return errors with offsets.
- UI work is checked with Playwright screenshots of every workspace in both themes.

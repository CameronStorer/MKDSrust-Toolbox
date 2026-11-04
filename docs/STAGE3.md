# Stage 3: real-hardware foundation in the app, plus textures & 2D at NitroPaint level

Read SPEC.md v0.2 first: the Toolbox is **real hardware first**. Everything it makes is a ROM hack.

**How to work:** breadth first. Get every item working end to end at a basic level before you polish any of them.
Reuse the architecture, keep the checks green, and leave changes uncommitted (the maintainer commits). If something
is blocked, write it in docs/STATUS.md and move on.

## A. Export for DS in the app
The core already exists: `toolbox_core::export::{build_rom, export_ds}`, `nitro_formats::{rom_build, bps}`, and the
CLI `mkdstb export-ds`.
- **Export dialog** (File > Export for DS, plus a big button on Overview):
  - output choice: "Patched ROM (for my DS / flashcart / emulator)" and/or "BPS patch (to share)";
  - file pickers; never allow writing over the clean ROM;
  - a result panel: changed files, in-place vs moved, patch size, warnings, and "Show in folder".
- **DS badge** in the top bar: becomes "DS OK" or "PC only (n)" from validation. Clicking it opens the Compatibility
  panel listing the issues (SPEC 3.2). Fix buttons come later; list them now.
- **Settings:** "Allow PC-only features" (off by default). While off, PC-only actions are hidden. Today that's
  nothing yet, so just add the setting and the plumbing.

## B. Open a ROM hack or patch as a project base
- **New Project from:** (a) the clean ROM (today), (b) a modded ROM (.nds), or (c) a patch (.bps; also IPS as a
  stretch goal) applied to the clean ROM.
- Store the base as a BPS in the project (`base.bps`, made with `bps::create` against the clean ROM for a modded
  ROM). project.toml gets `[base] clean_sha256 = ..., patch = "base.bps"`. Never store the modded ROM itself.
- The working ROM session is clean ROM + base patch. Edits apply on top. Export rebuilds from the base and writes
  the final BPS **against the clean ROM**.
- **Overview "What this hack changes" list:** files that differ between clean and base (added, removed, changed),
  with "changed code" for arm9/arm7/overlays.
- **Provenance:** a project made from someone else's hack records its file name and hash. Export warns that
  publishing needs the original author's credit or permission.

## C. Play targets (Test Bench)
- **Emulator:** a path in Settings (melonDS, DeSmuME or any executable). Play exports a temp patched ROM to the
  per-user cache and launches `<emulator> <rom>`.
- **MKDSrust:** as stage 2 (`--race` and so on).
- **Flashcart:** "Export to folder…" remembers the folder (e.g. the SD card).

## D. Textures & 2D at NitroPaint level
- **Pixel painting** on NCGR (tile) and NSBTX (texture) images:
  - tools: pencil, fill, line, rectangle, eyedropper, select/move;
  - palette-index aware (paint with palette entries);
  - zoom, grid, tile grid, undo/redo via the project history.
- **Import PNG:**
  - into an existing graphic with a palette: quantize to the palette's size (median cut or k-means plus dithering
    on/off; port NitroPaint's ideas, not its code);
  - into a texture: pick the format (4/16/256-colour, a3i5, a5i3, direct; 4x4 compressed as a stretch goal) and
    re-encode;
  - show a before/after preview, and the size and VRAM budget.
- **Export PNG** of any graphic or texture (exists for some; make it universal).
- **Cells (NCER) and animations (NANR):** view cell compositions over the NCGR+NCLR, list animation frames with
  playback. Editing cell positions is a stretch goal.
- **Recipes** (guided flows, a "?" panel): "Recolour a menu", "Replace an item icon", "Replace a kart texture".

## E. Done means
- All AGENTS.md rule 5 checks pass.
- New Rust code has tests on synthetic data.
- An `MKDS_ROM`-gated test: create a project from a synthetic "hack" (a BPS that changes one file), edit another
  file, export, and check the final BPS against the clean ROM reproduces both changes.
- Screenshots of the Export dialog, the "What this hack changes" list, and the painting tools (synthetic data).
- docs/STATUS.md updated.

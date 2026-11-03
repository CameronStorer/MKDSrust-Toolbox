# MKDSrust plugin / mod package format (v1)

Shared contract between the game (native FS overlay), the launcher (install/enable UI) and the site (upload screening).
Status: agreed baseline, 2026-10-28; format 2 (scripts) added 2026-11-03. Change only with all three sides updated.

## Package
A `.zip` (≤100 MB, ≤5000 files, ≤400 MB unpacked, paths ≤200 chars, no `..`, no absolute paths, no symlinks) with
`manifest.json` at the root:

```json
{
  "format": 1,
  "type": "character | track | kart | battle | music | texture | other",
  "id": "author.my-mod",            // 3-64 chars [a-z0-9._-], globally unique
  "name": "My Mod",
  "version": "1.2.0",               // semver-ish
  "author": "name",
  "game": "AMCE",                   // MKDS USA rev 0 (the only supported dump)
  "license": "CC-BY-4.0",           // optional
  "description": "optional, ≤2000 chars",
  "files": {                         // ROM path -> how to produce it
    "data/Course/cross_course.carc": { "patch": "patches/cross_course.carc.bsdiff" },
    "data/Scene/new_menu.bmg":       { "add": "files/new_menu.bmg" }
  },
  "depends": ["other.mod@>=1.0.0"],  // optional
  "conflicts": ["x.y"]               // optional
}
```

Two kinds of entries, nothing else:
- **`patch`**: a binary diff (bsdiff 4 format, `qbsdiff` crate compatible) applied to the file **from the player's own
  ROM**. This is the ONLY way to change an existing game file. The package must never contain the retail bytes.
- **`add`**: a brand-new file the mod author made entirely themselves (new path, not in the ROM's file table).

## Legal rules (enforced by site screening, re-checked by the launcher)
1. No ROMs, saves, executables or scripts (blocked extensions: nds srl dsi sav dsv gba 3ds cia nsp xci exe dll bat cmd com
   scr msi ps1 vbs sh jar app apk; DS ROM header detection on every file).
2. No Nintendo files, whole or modified: every packaged file's SHA-256 must not match any retail file hash, and an `add`
   file must not be "near-identical" to a retail file (same size ±5% and ≥50% equal 4 KiB blocks, or ≥50% matching
   rolling-hash chunks). Retail hashes come from `retail-index.json` (path, size, sha256, per-4KiB-block hashes) generated
   from the owner's ROM by `mkds_builder --retail-index`; it holds hashes only, never game bytes.
3. A `patch` target must be a real ROM path; the patch must not simply embed the target (patch size must be < 60% of the
   target size unless the target is < 64 KiB, flagged for manual review otherwise).
4. Every upload enters **quarantine**: automated checks first, then a moderator approves before it is public. Reports can
   pull a mod back into quarantine.

## Install layout (game folder)
```
<game>/mods/<id>/manifest.json + package contents (unpacked)
<game>/mods/enabled.txt            one id per line, load order top→bottom (later wins)
```
At start-up the game builds a virtual file table: ROM files, then for each enabled mod apply `patch` (to the current
bytes for that path, after earlier mods) or `add`. Patched results are cached in `<game>/mods/.cache/<hash>`.
A mod that fails (bad patch, missing dependency, conflict) is skipped with a log line; the game still starts.
`MKDS_NO_MODS=1` disables all mods. Mods never apply to the `--timing exact` acceptance run.

## Allowed file types (owner decision 2026-10-28)
Uploads may contain ONLY formats the native game reads (formats are fine; content must be the uploader's own work and
passes the retail near-identical check), plus package metadata. Everything else is rejected.
- Models/animation: .nsbmd .nsbtx .nsbca .nsbta .nsbtp .nsbma .nsbva (magic BMD0 BTX0 BCA0 BTA0 BTP0 BMA0 BVA0)
- 2D: .nclr .ncgr .nscr .ncer .nanr (magic RLCN RGCN RCSN RECN RNAN)
- Archives: .narc .carc (NARC; .carc = LZ77 0x10-compressed NARC, check after decompress)
- Sound: .sdat .sseq .sbnk .swar .strm (SDAT SSEQ SBNK SWAR STRM)
- Text: .bmg (MESGbmg1)
- Course: .kcl (no magic: validate header offsets) .nkm (NKMD)
- Package: manifest.json, README.md/.txt, preview .png/.jpg/.webp (≤5 MB each, ≤8), patches .bsdiff (BSDIFF40)
The magic must match the extension; nested archive contents are screened recursively with the same rules.

## Platform compatibility (owner decision 2026-11-02)
Manifest field `"platforms": ["pc"]` (default) or `["pc","ds"]` (author claims it also works on real DS hardware / emulators
when applied to the ROM). Badges shown on the site and in the launcher:
- **PC only**: default; forced when the mod uses PC-only features (future code plugins, widescreen/HD assets, files over DS
  limits).
- **DS compatible**: author declared `ds` AND automated checks pass: only `patch`/`add` of game data; NSBTX/NSBMD textures
  ≤1024 px per side and total texture data within DS VRAM budget (512 KiB textures + 96 KiB palettes per model set);
  resulting ROM ≤ 256 MiB; no PC-only resource types. Failing checks downgrade to PC only with the reasons shown.
- **Verified on DS**: set by moderators after the author or community submits a hardware/emulator test report.
Launcher (later): "Export for DS" applies enabled DS-compatible mods to the player's own ROM and writes a patched copy or
an xdelta/BPS patch for flash carts / emulators. Never distribute the patched ROM.

## Format 2: Luau scripts (2026-11-03)
`"format": 2` adds two manifest fields. Everything in format 1 still applies.
```json
"scripts": ["scripts/item_odds.luau", "scripts/hud.luau"],   // Luau source files in the package, run in this order
"api": 1                                                       // the script API version (required with scripts)
```
- A format 2 package may have an empty `files` (scripts only). Format 1 packages may not use `scripts` or `api`.
- Scripts are `.luau` UTF-8 source text: no NUL bytes, so no compiled bytecode; at most 256 KiB each and 32 per
  package; paths follow the usual rules.
- **Site:** every script puts the package in review, and a moderator reads it before approving.
- **Game:** each script runs sandboxed, with no files, OS or network, a CPU budget per frame and a memory limit.
  Scripts run only from enabled mods, in load order, and never in `--timing exact` acceptance runs.
- The API is documented in MKDSrust-Docs `4_docs/LUA_SCRIPTING_DESIGN.md`. Owner decisions: raw-memory access will be
  allowed in published mods with a badge; online play is out of scope.

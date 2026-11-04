# Toolbox status — 2026-11-03

Stage 4 A–E work at a basic end-to-end engine level. Stages 1–3 (projects,
archive edits, text/2D tools, packages, BPS/DS export and Play targets) remain available.

## Done

- Retail correctness: resource-specific dictionaries (also during texture replacement),
  compact PCMP palettes, legacy zero-BOM NCGR lengths and CLI test environment isolation.
  The owned ROM was read in place. Local tests round-trip 286 archives, 92 BMGs,
  433 NCLRs, 672 NCGRs, 470 NSCRs and 124 NKMs; parse 766 NSBMDs, 335 NSBTXs,
  136 NSBCAs and 59 KCLs. Counts print with `--nocapture`.
- Models: GLB/glTF and OBJ import; transforms, quadric decimation, interpolated plane
  cuts, part/material/connected/box deletion, seam-aware merging and normal recomputation.
  Authored NSBMD writes bones, inverse binds, weighted matrices, materials, GX triangles
  or connected strips, lookup trees and optional embedded textures. PNG/JPEG textures
  resize to capped powers of two and use the existing quantizer. Scene and badge
  validation share the model budget function.
- Collision: fixed-point prisms, conservative spatial octree, configurable material/slope
  classification and repainting. A retail rebuild retained all 902 prisms and a valid
  tree. MKDS type bits now use bits 8–12; observed types 0–22 live in `mkds-data`.
- Audio: SDAT SYMB/INFO/FAT/FILE, SSEQ, SBNK, SWAR/SWAV, SSAR and STRM readers/writers.
  The retail SDAT and its 76 sequences, 99 banks, 104 wave archives and 5 sequence
  archives round-trip; relocation and a short retail sequence preview also pass.
  WAV resampling, mono/stereo, PCM8/PCM16/IMA-ADPCM, aligned loops, independent sampled
  song banks, RAM reporting and sample replacement. MIDI imports notes, tempo, programs,
  volume, pan, pitch bend and loop markers, mapping programs against a chosen bank.
- App: model import, transform gizmos, part selection, decimation/cut/delete/merge and
  budgets; Track collision generation, attribute colours and stroke painting; Audio
  tree, song import options and software playback. Mutations use project history and
  archive rebuilding. Synthetic tests cover save/reopen, undo/redo, packing and BPS
  reproduction across all engines. IPC bindings and original demos are regenerated.

## Partial / next

- FBX/DAE are deferred: no suitable importer was cached. Convert in Blender to GLB/glTF/
  OBJ. glTF requires dense triangles and a TRS skeleton whose rest pose matches its
  inverse binds. Sparse/Draco accessors, arbitrary bind-pose conversion and OBJ MTL
  image loading remain future work. PNG/JPEG textures cover the glTF image formats.
- The writer rejects models above 31 usable matrix slots; automatic splitting and
  rig fitting remain next work. Cuts are uncapped; decimation can stop above a target
  when safe collapses are unavailable. KCL candidate lists favour correctness over size.
- Audio preview is basic, without accurate envelopes or every SSEQ command; unsupported
  commands report errors. WAV accepts PCM8/PCM16/float32; MIDI accepts formats 0/1 with
  PPQN timing. Sampled-song allocation is capped at 2 MiB, not a full live RAM budget.
- Next: Stage 5 research/wizards and emulator/DS boot tests. No hardware or external game
  install was launched. Inherited limits include 4×4 texture encoding, IPS/xdelta,
  new top-level ROM files/FNT reconstruction, detailed cell affine/transfer policies
  and editing non-joint animation formats.

## Verification and run

Passed: `cargo fmt --check`, workspace Clippy/all targets with `-D warnings`, workspace
tests, and `cargo test --workspace --release --no-fail-fast` with `MKDS_ROM` set to the
external dump in `STAGE4.md`. App lint/typecheck, 6 Vitest tests, production build and
Tauri `cargo build` pass. All 30 Playwright tests pass, including actual gizmo dragging
and collision brush strokes. Screenshots use generated fixtures in both themes.
No Nintendo data was added; the clean ROM was never copied into the workspace.

From the repository root in PowerShell, set `$env:CARGO_HOME="$PWD/.cache/cargo"` and
`$env:CARGO_NET_OFFLINE='true'`. With Rust stable, Node 22 and Tauri prerequisites,
in `app/` run `npm ci`, then `npm run tauri dev`. Open the external AMCE rev 0 ROM
and a project. Select an existing model/course/SDAT and use Model, Track or Audio.
Imports replace selected resources; undo and export work as before. Audio loop fields
use source sample numbers; MIDI loop markers are `loopStart` and `loopEnd`.

Local counts: set `MKDS_ROM`, then run
`cargo test -p toolbox-core --release --test retail_opt_in -- --nocapture`.
Regenerate original demos: `cargo run -p toolbox-core --example demo`.
For browser checks in `app/`, set `PLAYWRIGHT_BROWSERS_PATH=<repo>/.cache/playwright`
and `MKDSTB_SCREENSHOT_PREVIEW=1`, then run `npm run test:e2e`.
The only added crate is `jpeg-decoder` 0.3.2 (MIT OR Apache-2.0), without default
features or transitive dependencies. It is cached; all final builds ran offline.

Changes are uncommitted. Nothing was pushed; Git metadata/remotes and the builder
repository were not modified.

# Toolbox status — 2026-11-04

Stage 5 A–E have basic end-to-end authoring paths. F has automated export/readback
and synthetic screenshots; current-output boot verification remains with the maintainer.
Stages 1–4 remain available. Changes are uncommitted.

## Done

- A: bounded BLZ, ARM9 module parameters, typed RAM-addressed tables, overlay
  recompression/fallback and an uncompressed option for either binary. Overlay storage
  policy is project metadata, never a ROM member.
- B: character slot/base selection, automatic orientation with up-axis override,
  hip cut, fitting, reduction, donor skeleton/rigid assignment and paint brush,
  race/menu/E models, donor clips, seats and donor race-voice routing. Seated
  reference/animation preview; rendered or PNG portraits into a chosen cell.
- C: kart body/texture/shadow generation, donor stats and all thirteen seats,
  tires, wheel coordinates/picking, placement gizmos with seated reference,
  ground/centre helpers and speed/acceleration forms.
- D: material/slope collision, lighting defaults, closed racing-line generation,
  terrain-height placement, 64 checkpoints, CPU/item routes, starts, respawns,
  camera, item rows, minimap and D variants. Editable points, route repair and
  collision compaction Fix buttons. Unrouted objects copy donor MapObj families.
- KCL: 1024-unit cubic root grid, coordinate shift 10, nonzero Y/Z strides,
  conservative candidates, shared lists and bounded coarsening. Warn above
  **150,000 bytes**, refuse above **215,000 bytes**, including export validation.
  The 6,272-prism / 2,000-unit dense fixture is **173,740 bytes**; vertex/centroid
  lookups retain candidates across root boundaries. Larger geometry is refused.
  Every generated track member also stays within the largest matching expanded
  retail member, measured in memory; manual edits are checked again at export.
- E: WAV/MIDI music assignment with RAM reporting, allocation-bounded sound
  replacement, cup order/name-message updates, HUD cells, embedded item retexturing
  and searchable grouped names. Each recipe is one undoable command.
- F: synthetic persistence/undo/BPS and opt-in owned-ROM export/readback tests:
  character, kart, both course-20 variants (14 files), music, effects, icons,
  embedded textures, names, cups and uncompressed overlays. Original hash checked.
  All eight recipe screens captured in both themes: `docs/screenshots/recipe-*`.

## Limits / remaining verification

The maintainer boot-tested earlier slot-0 character/kart outputs and the coarse KCL
track fix. **The new root grid, donor voices and current exports need boot tests**;
no DS hardware/emulator verification is claimed. Native-builder dependency resolution
was attempted with this task's offline CARGO_HOME: `corosensei` is not cached.

Portrait destination/crop is explicit; localized selection/HUD/result cells are not
all auto-mapped. Tracks supply one camera and no area triggers: inspect intro/goal
views and retained minimap backgrounds. Height placement uses the highest XZ surface;
bridges/tunnels need manual edits. Routed donor objects need PATH/POIT wiring.
Automatic orientation uses the longest source axis. Inherited limits include FBX/DAE,
arbitrary animation retargeting, 4x4 texture encoding, IPS/xdelta and new top-level
ROM files. These are not claimed complete.

## Verification and run

Passed: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` (**79 tests with MKDS_ROM set**), app lint/typecheck,
6 Vitest tests, production build, Tauri `cargo build`, and **46 Playwright tests**.
No Nintendo content was added. The owned ROM was read in place; the new retail tests
keep rebuilt images in RAM. Demo assets/screenshots are generated synthetic data.

PowerShell from root: set `$env:CARGO_HOME="$PWD/.cache/cargo"` and
`$env:CARGO_NET_OFFLINE='true'`. In `app/`, run `npm run tauri dev`; open the external
AMCE revision-0 ROM and a project, then **Beginner recipes**. Apply, inspect and export
outside the repository. Set `MKDS_ROM` from `docs/STAGE4.md` for local tests.
Regenerate demos: `cargo run -p toolbox-core --example demo`. For browser checks,
in `app/` set `PLAYWRIGHT_BROWSERS_PATH=<repo>/.cache/playwright` and
`MKDSTB_SCREENSHOT_PREVIEW=1`, then `npm run test:e2e`.

For each patched output, the maintainer's exact MKDSrust boot-test sequence is:

```text
mkds_builder <patched.nds> --build --allow-unverified --out <dir>
<dir>/game.exe --assets <dir>/assets --race course=20,character=0,kart=0,racers=8 --frames 1500
```

Use an empty output directory outside this repository. Repeat for character, kart,
track, music/effects and items. For cups/names also run
`<dir>/game.exe --assets <dir>/assets --list-courses` and inspect the renamed cup and
race order in the game menu. Automated tests independently check tables/messages;
a course listing alone does not verify every localized string.

No Git metadata/remotes or builder sources were modified; nothing was pushed.

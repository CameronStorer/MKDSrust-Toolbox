# MKDSrust Toolbox

A real-hardware-first, all-in-one ROM-hacking studio for **Mario Kart DS**: tracks, karts,
characters, textures, text, audio and code. Everything it makes plays on a real DS, flashcart or emulator, and
optionally on PC through [MKDSrust](https://mkdsrust.com). Goal: even a beginner can make a ROM hack.

> **Status:** Stages 1–3 implemented at a basic end-to-end level. See [docs/STATUS.md](docs/STATUS.md) for verification and limitations.

- Bring your own legally dumped ROM. This project contains **no Nintendo material** and never will.
- Mods are exported as MKDSrust packages (binary patches against your own files), see
  [docs/PLUGIN_FORMAT.md](docs/PLUGIN_FORMAT.md).
- Export patched ROMs, BPS, PNG, JSON, GLB and OBJ. MIDI/WAV authoring is later work.

Built with Rust + Tauri, React and three.js. The later Code workspace adds Monaco. Inspired by the community's tools, see
[docs/PRIOR_ART.md](docs/PRIOR_ART.md) and [CREDITS.md](CREDITS.md).

## Run

Install Rust stable, Node 22, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/).
On Linux, install WebKitGTK 4.1, OpenSSL and the other packages listed in `.github/workflows/ci.yml`.

```sh
cd app
npm ci
npm run tauri dev
```

Choose your own AMCE USA revision 0 ROM, then create a project in a separate folder. Browse archives in Explorer;
double click an NCLR or BMG to edit it. Pair NCGR tiles with an NCLR and optional NSCR in Texture & 2D. Edits save
immediately and share a project-wide undo history. Overview edits the package metadata and packs
`build/<id>-<version>.zip`, ready for the launcher's existing mod installer. Your original ROM is never changed.

Overview and File > Export for DS write a patched ROM for local play and/or a shareable BPS. New Project can use
a modded ROM or BPS as its base; it stores `base.bps` and provenance, and exports the final patch against the clean
ROM. Test Bench targets a configured DS emulator, an MKDSrust portable install, or a flashcart folder. MKDSrust
race setup uses the install's course list, numeric character/kart ids and 1–8 racers; Reload restarts the race.

Model and Track open NSBMD/NKM/KCL from Explorer, with skinned animation previews, overlays and budgets.
Explorer exports GLB and OBJ. Texture & 2D paints raw NCGR/NSBTX pixels; each stroke and PNG import is undoable.
PNG conversion previews median-cut palettes and optional dithering in six DS texture formats. NCER/NANR open
cell compositions and animation playback using companion NCGR/NCLR files. The ? Recipes panel guides common edits.

```sh
cargo run -p toolbox-cli -- ls /path/to/your.nds
cargo run -p toolbox-cli -- extract /path/to/your.nds data/path/archive.carc/member.nclr -o /outside/project/palette.nclr
cargo run -p toolbox-cli -- convert /path/to/palette.nclr --to png
cargo run -p toolbox-cli -- convert /path/to/tiles.ncgr --to png --palette /path/to/palette.nclr
cargo run -p toolbox-cli -- pack /path/to/project
cargo run -p toolbox-cli -- validate /path/to/project
cargo run -p toolbox-cli -- export-ds /path/to/project --rom /outside/project/patched.nds --bps /outside/project/hack.bps
```

JSON conversion supports BMG, NCLR, NCGR, NSCR and NSBTX metadata. Palette/tile/screen/texture previews export to PNG.
The typed command bindings can be regenerated with `npm run bindings` in `app/`.

## Verify and capture the UI

```sh
cd app
npm run lint
npm run typecheck
npm test
npm run build
npx playwright install chromium
npm run test:e2e
cd ..
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --manifest-path app/src-tauri/Cargo.toml
```

The browser script starts Vite with a visibly marked synthetic backend, exercises the editing workflow, and
captures all ten workspaces in both themes under `docs/screenshots/`. Regenerate its fixture metadata/image with
`cargo run -p toolbox-core --example demo`. No ROM is needed for any normal check. For the optional local ROM test,
set `MKDS_ROM` to your own dump and run `cargo test -p toolbox-core --test retail_opt_in --test stage3 -- --nocapture`.
Local export tests use temporary directories outside the repository and remove them on completion.

Add `-- --pack` to the demo command to generate `.cache/demo-mod.zip` for package interoperability checks.
PC-only exports omit the optional `platforms` field (the v1 default is PC) for compatibility with older launchers.

If a restricted environment prevents Vite's native dependency optimizer from reading ancestor directories,
`MKDSTB_SCREENSHOT_PREVIEW=1 npm run test:e2e` uses the same synthetic UI through Vite's production preview instead.
On Windows PowerShell, set `$env:MKDSTB_SCREENSHOT_PREVIEW='1'` before running that command. `MKDSTB_REUSE_SERVER=1`
allows an explicitly managed existing synthetic server during local debugging.

License: [GPL-3.0-or-later](LICENSE).

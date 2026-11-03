# MKDSrust Toolbox

A pro-grade, all-in-one modding studio for **Mario Kart DS** on [MKDSrust](https://mkdsrust.com): tracks, karts,
characters, textures, text, audio and code, with a one-click Test Bench that races your mod in your own MKDSrust install.

> **Status:** Stage 1 foundation implemented. See [docs/STATUS.md](docs/STATUS.md) for verification and limitations.

- Bring your own legally dumped ROM. This project contains **no Nintendo material** and never will.
- Mods are exported as MKDSrust packages (binary patches against your own files), see
  [docs/PLUGIN_FORMAT.md](docs/PLUGIN_FORMAT.md).
- Stage 1 exports PNG and JSON. Later stages add glTF, MIDI, WAV and OBJ.

Built with Rust + Tauri and React. Later workspaces add three.js and Monaco. Inspired by the community's tools, see
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

```sh
cargo run -p toolbox-cli -- ls /path/to/your.nds
cargo run -p toolbox-cli -- extract /path/to/your.nds data/path/archive.carc/member.nclr -o /outside/project/palette.nclr
cargo run -p toolbox-cli -- convert /path/to/palette.nclr --to png
cargo run -p toolbox-cli -- convert /path/to/tiles.ncgr --to png --palette /path/to/palette.nclr
cargo run -p toolbox-cli -- pack /path/to/project
cargo run -p toolbox-cli -- validate /path/to/project
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
set `MKDS_ROM` to your own dump and run `cargo test -p toolbox-core --test retail_opt_in -- --nocapture`; that test
keeps all ROM-derived bytes in memory.

Add `-- --pack` to the demo command to generate `.cache/demo-mod.zip` for package interoperability checks.
PC-only exports omit the optional `platforms` field (the v1 default is PC) for compatibility with older launchers.

If a restricted environment prevents Vite's native dependency optimizer from reading ancestor directories,
`MKDSTB_SCREENSHOT_PREVIEW=1 npm run test:e2e` uses the same synthetic UI through Vite's production preview instead.
On Windows PowerShell, set `$env:MKDSTB_SCREENSHOT_PREVIEW='1'` before running that command. `MKDSTB_REUSE_SERVER=1`
allows an explicitly managed existing synthetic server during local debugging.

License: [GPL-3.0-or-later](LICENSE).

# Instructions for coding agents

Read `docs/SPEC.md` (product and architecture), `docs/PLUGIN_FORMAT.md` (mod package contract) and
`docs/PRIOR_ART.md` first. Read `docs/STATUS.md`, then the current task: `docs/STAGE5.md` (stages 1-4 are done).

Hard rules:
1. **No Nintendo material, ever.** Do not add any ROM, game file, extracted asset, text, sound, or SDK code/binary. Tests
   use synthetic fixtures you generate in code. Tests that need the user's ROM read the path from the `MKDS_ROM` env
   var and are skipped (not failed) when it is unset; they must never write ROM-derived bytes into the repo.
2. Only copy code from projects with a GPL-3-compatible license (MIT, BSD, 0BSD, Apache-2.0, GPL-3), credit it in
   `CREDITS.md`, and keep its header. Same-author code from `C:\Users\cam\dev\mkds-native-builder` (GPL-3.0) may be
   ported freely: `1_build_tool/src/dsfiles.rs` (LZ77 incl. `lz77_recompress`/splice, NARC), `rom.rs` (ROM header,
   FNT/FAT), `modding.rs` + `modpkg.rs` (package format, bsdiff via `qbsdiff`). Do NOT modify that repo.
3. Don't push, publish releases or change git remotes. Your sandbox cannot write `.git`: leave changes uncommitted; the maintainer commits them.
4. Keep it cross-platform (no Windows-only APIs). Rust stable, edition 2021. TypeScript strict.
5. Before declaring done: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, and in `app/`: `npm run lint`, `npm run typecheck`, `npm test`, `npm run build`, and
   `cargo build` of the Tauri app must all pass. Write a short `docs/STATUS.md` with what is done, what is not, and how
   to run it.

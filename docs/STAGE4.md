# Stage 4: retail correctness, then the content engines (models, collision, audio)

Read SPEC.md v0.2, especially section 4b (the beginner path). This stage builds the engines the wizards will use.
The wizard UIs come in stage 5, once `docs/research/` maps the game's character, kart and track data.

**How to work:** breadth first. Each engine works end to end, with tests, before any polish. Keep the AGENTS.md rule 5
checks green, and leave changes uncommitted.

## A. Retail correctness first
The user's own ROM is available **for local tests only**: set `MKDS_ROM=C:\Users\cam\AppData\Local\Temp\claude\--192-168-1-11-Shared-Coding-My-NDS-GAME\d2702baf-4324-41bc-919f-6df583f54ea0\scratchpad\rom.nds`.
Read it in place. Never copy it, never put ROM-derived bytes into the repo, tests, fixtures or screenshots.
- `cargo test --workspace --release --no-fail-fast` with MKDS_ROM set must pass. It fails today:
  - NSBMD: "unexpected dictionary entry size" on `data/Course/Award.carc/course_model.nsbmd` (and maybe others).
    Nitro dictionaries have per-resource entry sizes; check every NSBMD/NSBTX/NSBCA in the ROM parses.
  - NCLR: "truncated data" in `user_rom_roundtrips_and_palette_patch` (retail NCLRs vary: 4/8 bpp, extended
    palettes, PCMP chunks, odd sizes).
  - `toolbox-cli/tests/cli.rs` fails when MKDS_ROM is set. Make it independent of that variable.
- Round-trip every NARC, BMG, NCLR, NCGR, NSCR and NKM in the ROM. Parse every NSBMD, NSBTX, NSBCA and KCL. Report
  counts in the test output.

## B. Model engine (crates: nitro-formats `model` + toolbox-core `model`)
- **Import:** glTF/GLB (primary), OBJ, and FBX/DAE via a pure-Rust or WASM-free library if one fits the license and
  the offline cache. Otherwise glTF/OBJ now and FBX later (note it in STATUS). Produces the internal `Mesh`:
  positions, normals, UVs, colours, materials, skin.
- **Operations:**
  - transform (move, rotate, scale, apply);
  - **quadric decimation** to a target triangle count;
  - **cut by plane** (keep one side, cap optional), for "remove the legs";
  - delete by selection (material, connected part, box);
  - merge vertices;
  - recompute normals.
- **NSBMD writer:**
  - build the model, materials, textures (NSBTX inside or separate), display lists (GX commands: triangles and
    strips, normals or vertex colours, UVs), nodes/bones and matrix stack;
  - a skinned model with N bones;
  - the round trip `parse(write(mesh))` must match the mesh.
- **Budgets:** polygon, vertex, texture-VRAM and palette counts for a mesh plus its textures, against the DS limits
  in SPEC 4. One function the DS badge uses.
- **Texture conversion** for models: any image to a DS texture format (reuse the stage 3 quantizer), with the size
  snapped to powers of two and capped, plus the budget.

## C. Collision engine (KCL)
- **KCL writer** from a triangle mesh with a per-triangle attribute: prisms plus the octree, matching the game's
  layout (study `nitro-formats::kcl` and retail KCLs). Round trip with the parser. Build a retail course's KCL from
  its own prisms as a test, and compare structure (prism count and octree validity, not bytes).
- **Auto-classify** triangles from slope (floor/wall) and material names ("road", "grass", "water", "lava",
  "fall"...). Use a configurable attribute table. Attribute values will come from `docs/research/`; until then,
  read the values present in retail KCLs and keep them in `mkds-data`.

## D. Audio engine (SDAT family)
- **Parse and write SDAT:** SYMB/INFO/FAT/FILE, plus SSEQ, SBNK, SWAR/SWAV, SSAR, STRM. Round-trip
  `sound_data.sdat` from the ROM.
- **WAV import:** resample, mono/stereo, encode to IMA-ADPCM or PCM16/PCM8 as an SWAV with loop points, and build a
  bank plus a one-note looping sequence so it plays as music (the "sampled" route). Report its RAM size.
- **MIDI import:** MIDI to SSEQ (notes, tempo, program changes, volume, pan, pitch bend, loops via markers). Map
  General MIDI programs to a chosen SBNK.
- **Playback preview** in the app: a simple software mixer for SWAV/SSEQ. Basic is fine.

## E. App
- **Model workspace:** import a model, then transform with gizmos, decimate (slider plus a live triangle count),
  cut plane, delete parts, and see the budget readout.
- **Track workspace:** "Generate collision" with attribute colours and a brush to repaint triangles.
- **Audio workspace:** the SDAT tree, play sequences and samples, "Import song" (WAV/MIDI) with loop points and the
  route choice.

## F. Done means
- Every rule 5 check passes, **and** `MKDS_ROM`-gated tests pass locally.
- Synthetic round-trip tests for the NSBMD writer, KCL writer and SDAT writer.
- STATUS.md updated (done / partial / next).

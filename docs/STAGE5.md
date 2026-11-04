# Stage 5: the beginner wizards (character, kart, track, music, cups, items)

Read SPEC.md section 4b (the owner's walkthrough) and `docs/research/CHARACTERS_KARTS.md` +
`docs/research/TRACKS_AUDIO_CUPS.md`. The research tells you exactly what the game needs. Where it says "inferred" or
lists an open question, verify it against the ROM (MKDS_ROM, read in place, same rules as stage 4) before you rely on
it, and correct the research doc if it's wrong.

**How to work:** breadth first. Each wizard produces a working result end to end (export for DS, then the patched
ROM boots in MKDSrust `--race` with the change visible) before any polish. Leave changes uncommitted. Keep the checks
green.

## A. ARM9 and overlay editing (needed for cups and music)
- BLZ (backward LZ) decompress plus compress for ARM9 and overlays, with the module-params fields kept consistent.
  Offer "store uncompressed" (set the compressed-end field to 0; hardware accepts it), and use it if recompression
  would grow past the original slot.
- A typed table editor over the decompressed binary by RAM address (from `mkds-data`). The rebuilt ROM writes the
  binary back through `rom_build`.

## B. Character wizard (replace a slot; adding a 14th is out of scope)
Following the research recipe:
1. pick a slot (e.g. Shy Guy) and a **base character** for skeleton, animations and voice;
2. import a model;
3. auto-orient and scale;
4. cut plane at the hips (preview seated);
5. decimate to the measured driver budget;
6. **auto-rig**: copy the base skeleton exactly, assign each vertex to its nearest bone region (rigid, one bone per
   vertex), with a paint-to-reassign brush;
7. textures to 16-colour within the measured limits;
8. generate every file the research lists (`P_xx` race and menu, `E_xx`, shadow) and keep the base's clips;
9. icons rendered from the model, or an image.

Preview: play the base's drive, spin, win and lose clips on the new model.

## C. Kart wizard (replace a kart id)
1. import a model;
2. **placement view** with the base driver seated: gizmo, snap to the ground, "centre under driver";
3. decimate and convert textures (`kart_body`, `kart_emblem` materials as the research describes);
4. **wheels**: click 4 wheel centres (or auto-detect separate wheel meshes) and pick the tire size S/M/L, written to
   `kartoffsetdata.bin`, with the seat offsets for every character;
5. stats from a base kart (`kartphysicalparam`...), as a simple form.

## D. Track wizard (replace a course)
1. import a map model;
2. **collision** with the stage 4 engine plus the research's attribute values (road, offroad, wall, boost, fall-off,
   wading...) and brush fixes;
3. a default light;
4. **draw the racing line** (top-down polyline). Generate the start (KTPS), at least 16 checkpoints (CPOI/CPAT),
   CPU routes (EPOI/EPAT), item routes (IPOI/IPAT), respawns (KTPJ) and a camera, so every required NKM section
   exists. Every point stays editable;
5. **item box rows** along the line; the object palette shows the item box first (safe anywhere), and other objects
   once their files are copied in (follow the research's object families);
6. **minimap**: render the top view into `Map2D/local.*` (find the world-to-pixel transform: open question);
7. the course model with budgets from the research; sky optional (keep the base course's);
8. validation with Fix buttons (missing sections, too few checkpoints, routes not closed, out-of-budget).

## E. Music, sound effects, cups, items, text
- **Course music:** import a song (stage 4 audio) and assign it to a course by writing the music slot table (A).
  Show the RAM estimate against the sound heap (about 700 KB; aim under 250 KB of samples).
- **Sound effects:** browse the SDAT effects and replace one with an audio clip.
- **Cups:** a cup editor. Pick 4 courses per cup (cup table, A); course and cup names come from the text messages
  in the research. Cup name sprites use the 2D tools.
- **Items:** "Replace an item icon" (HUD sprite) and "Recolour or retexture an item" (item model textures).
- **Text:** a "names" view grouping course, cup, character and kart names, plus search.

## F. Done means
- Checks green.
- MKDS_ROM tests: for each wizard, build a project from a synthetic model, mesh or audio input, export, re-open the
  patched ROM, and check the expected files and tables changed and still parse.
- For each wizard, `mkds_builder --race` boots the patched ROM in MKDSrust with the change visible (characters,
  karts, tracks), or `--list-courses` shows it (cups and names). If the MKDSrust build of a patched ROM isn't
  possible from your sandbox, write the exact command in STATUS.md for the maintainer to run.
- Screenshots of each wizard (synthetic data). STATUS.md updated.

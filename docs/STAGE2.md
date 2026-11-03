# Stages 2 + 3: 3D viewing, Track workspace (read-only), Test Bench (task for the implementing agent)

**How to work (token budget):** cover as much of this list as possible in one run. Work breadth first:
- get each feature working end to end at a solid basic level before you polish any of them;
- reuse the stage 1 architecture (commands, panels, undo, mock backend), don't redesign it;
- don't rewrite working code;
- keep the verification suite green as you go, rather than running every check after every small edit;
- commit after each feature.

If something is blocked, note it in docs/STATUS.md and move on to the next item.

## A. `nitro-formats`: 3D formats (parse; round-trip where marked)
- **NSBMD** (BMD0): model, bones/nodes, materials, polygon display lists (GX commands to triangles, with normals,
  UVs, vertex colours), matrix stack and skinning info. Parse and decode to a mesh.
- **NSBTX** (already read-only): wire textures to materials.
- **NSBCA** (BCA0): joint animations, decoded to per-frame TRS. **NSBTA/NSBTP/NSBMA**: parse the headers and list the
  animations (decoding is a stretch goal).
- **NKM** (NKMD): every section (OBJI, PATH, POIT, STAG, KTPS, KTPJ, KTP2, KTPC, KTPM, CPOI, CPAT, IPOI, IPAT, EPOI,
  EPAT, MEPO, MEPA, AREA, CAME), parse + write, exact round-trip.
- **KCL**: header, positions, normals, prisms, the octree. Parse, and turn prisms into triangles with their collision
  attribute.
- Synthetic round-trip tests for NKM and KCL. A small synthetic NSBMD/NSBCA built in the test from GX commands.

## B. `toolbox-core`
- `model::Mesh` (positions, normals, uvs, colours, indices, material, skin weights), from NSBMD; `scene::CourseScene`
  combining the course model, the KCL mesh and the NKM objects for one course.
- **glTF 2.0 export (GLB)** of an NSBMD with textures, skeleton and the NSBCA animations. **OBJ export** of KCL with
  one material per collision type.
- **Budget readout:** polygons, vertices, texture VRAM, palette VRAM, matrix count per model/course, against the DS
  limits in SPEC 3.2.

## C. App
- **Viewport** (three.js, reusable panel): orbit/pan/zoom (Blender-style default, MKTB-style option), frame
  selected, grid, wireframe/solid/textured, a camera speed setting, an FPS/poly counter overlay.
- **Model workspace (read-only for now):** open any NSBMD from the Explorer. Show it textured and skinned, with an
  animation picker and timeline. Properties panel lists materials, bones and budget.
- **Track workspace (read-only):** pick a course, then show the course model, plus overlays, each toggleable in an
  Overlays panel with visibility/lock and colour:
  - KCL coloured by collision type (with a legend);
  - CPU routes (EPOI/EPAT, MEPO/MEPA);
  - checkpoints (CPOI/CPAT, drawn as gate pairs);
  - item routes (IPOI/IPAT);
  - respawns (KTPJ), start positions (KTPS);
  - objects (OBJI as labelled boxes with their object id);
  - areas (AREA), cameras (CAME).

  Clicking an object shows its fields in Properties. The Outliner lists every NKM entry by section. The status bar
  shows polygon counts and validation issues (missing respawns, unlinked paths).
- **Explorer:** "Open in Model / Track workspace" for the right files. Export glTF/OBJ from the context menu.
- **Test Bench workspace (stage 3):**
  - find the MKDSrust install: portable launcher folder `data\` + `games\`, set in Settings;
  - **Play** in the top bar (enabled now) packs the project and installs the package into
    `games/<game>/mods/<id>/` (unpack, add the id to `mods/enabled.txt`, replacing an older copy);
  - start the game executable from the install;
  - stream its stdout/stderr into a Log panel;
  - Stop button;
  - a "last run" summary (package size, install path, exit code).
  - **Race setup (the game supports it now):** the game executable accepts
    `--race course=<id|name>,character=N,kart=N,racers=N` (any subset) and starts that race directly,
    with no menus (about half a second after boot). `--list-courses` prints `id<TAB>internal_name` lines from the
    player's own game and exits (run it with `--headless --instructions 20000000 --mute`). Build the course picker
    from that list (cache it per install). Character/kart pickers take ids for now. Racers 1-8. Engine class isn't
    supported yet (show the picker disabled).
  - **Reload:** in-place asset reload isn't supported by the game yet. "Reload" = Stop, re-pack/install, and start
    again with the same `--race` (it's fast). Bind it to Ctrl+R in the Test Bench.
- Update the Playwright screenshots (mock backend with synthetic data): Model, Track (with overlays on), Test Bench.

## D. Done means
- All AGENTS.md rule 5 checks pass.
- New formats have synthetic round-trip tests.
- `MKDS_ROM`-gated test: opens every NSBMD/KCL/NKM in the ROM, parses all of them, round-trips every NKM, and
  exports one course to GLB/OBJ into a temp dir (never into the repo).
- docs/STATUS.md updated: what works, what is partial, what is next.

# Mario Kart DS (USA, AMCE): playable characters and karts, data structure

Structure only. No game bytes, textures or text are reproduced.
Status tags: **[C]** confirmed (measured in the ROM, or read in the translated source), **[I]** inferred.

Evidence used: the ROM file listing; parsing of every NSBMD/NSBCA/NSBTX in the four kart archives; the decompressed ARM9 (string and pointer tables); the translated source in `ae/source/code_kart_*`, `code_menu_*`, `code_sys_fs`, `code_save`; the dictionary `game_names.csv`; and `mkds-docs/content/analysis/CODEX_ANIMATION_MAP.md`.
Web: [Course Modifier / NSBMD notes](https://xortroll.github.io/posts/nitroedit/), [GBAtemp "Importing models into MKDS"](https://gbatemp.net/threads/importing-models-into-mkds-tutorial.543796/), [format overview](https://fileinfo.com/software/mkds_course_modifier/mkds_course_modifier). I found no public documentation of bone names or CTGP-Nitro custom characters. Everything below comes from the ROM.

**Toolbox bug found.** `nitro-formats` `texture.rs::dictionary()` computes the entry table as `8 + u16(d,6)`. On every retail NSBMD, NSBCA and NSBTX the u16 at +6 is already the absolute offset of the entry table, so retail parsing fails with "unexpected dictionary entry size". All numbers here come from a scratch copy patched to `table = u16(d,6)`. This is the Stage-4 "retail correctness" item.

## 1. Identifiers

### Character ids (13)
Order of the table at 0x02155CEC (stride 12). [C]

| id | code | dir | id | code | dir |
|---|---|---|---|---|---|
| 0 | MR | mario | 7 | LG | luigi |
| 1 | DK | donkey | 8 | KA | karon (Dry Bones) |
| 2 | KO | kinopio | 9 | DS | daisy |
| 3 | KP | koopa | 10 | WL | waluigi |
| 4 | PC | peach | 11 | RB | robo |
| 5 | WR | wario | 12 | HH | heyho (Shy Guy) |
| 6 | YS | yoshi | | | |

Each row holds three string pointers: `P_xx` (detailed rigged driver), `E_xx` (static low-detail driver), and the code `xx`. The directory table is at 0x02165E80. The `character/` and `kart/` root strings are referenced at 0x02165AC4 and 0x02165AC8.

`OY` (oyama) exists only in the unused `KartModelMain.carc`, as a developer leftover. [C]

### Kart ids (36 used, plus a 37th)
The name table at 0x02155D8C has 8-byte entries. The first pointer is the kart model name; the second repeats the next entry's name (purpose unknown). [C]

- `id = 3*g + k`, where g is the group 0..11 in the order MR, DK, KO, KP, PC, WR, YS, LG, KA, DS, WL, RB (HH has no group).
- k=0 is always `MR_a`, the shared "Standard" model. k=1 is `XX_b`, k=2 is `XX_c`.
- Unique kart models: 1 (`a`) + 12 (`b`) + 12 (`c`) = 25, addressed by 36 ids.
- Standard-kart recolour per character comes from `kart/palette/kart_a.NCLR` [C file; per-character slot is **I**]. `kart_upload_palette` (0x02079443) divides the id by 3 and uploads a 16-colour palette found under the name "kart_body". [C]
- Id 36 is the 37th entry in the offset, appearance and weight tables. It is the only kart available to HH (see ktbl). [C]

Tire class by id is `id%3`: 0 is M, 1 is S, 2 is L. [C, from `kartoffsetdata.bin` names]

Weight class: `kart_get_weight_class` (0x0202578C) is a 37-entry jump table returning 0..2, with 3 for out-of-range ids. I did not map it to named classes. [C structure]

## 2. Archives and file inventory

The race loads `KartModelMainA.carc`, `KartModelMainB.carc` and `KartModelSub.carc`. The menu uses `KartModelMenu.carc` (mounted as `/data/KartModelMenu/...`). These path strings are in ARM9. [C]

`KartModelMain.carc` is **not referenced** by the code (no such string without the A/B suffix). It is a superset left over from development, containing OY and HH karts. [C]

| Archive | Contents |
|---|---|
| **KartModelMainA** | MR/YS/WR/PC/KP/KO/DK: `kart/<dir>/kart_XX_{b,c}.nsbmd/.nsbtx` (MR also `_a`). Also `E_xx` static drivers, `emblem/*_emblem.nsbtx` (15), `kart/tire/kart_tire_{S,M,L}`, `kart/shadow/sh_*`, `kart/palette/kart_a.NCLR`, `emblem_s_b.NCLR`, `character/common/P_faceanim.nsbtp`, `KillerItem*` (Bullet Bill), and the data bins. |
| **KartModelMainB** | The same for WL/RB/LG/DS/KA, plus the Shy Guy set: `E_HH`, `P_HH`, `P_HH_{drive,lose,spin,win}.nsbca`, `HH_color.NCLR`. |
| **KartModelSub** | 12 detailed drivers: `character/<dir>/P_XX.nsbmd/.nsbtx` plus `P_XX_{drive,spin,win,lose}.nsbca` (48 clips). HH is not here. |
| **KartModelMenu** | Menu versions: `P_XX.nsbmd/.nsbtx`, `P_XX_win.nsbca`, kart models, higher-detail tires (96 verts), shadows, `kart/select/select.nsbmd/.nsbtx` (floor ring), `kartmenuparam.bin`, `charmenuparam.bin`, `kartoffsetdata.bin`, `kartphysicalparam.bin`. |

Data bins (in MainA and Menu, same names): [C sizes]

| File | Size | Structure |
|---|---|---|
| `kartoffsetdata.bin` | 8288 = 37 x 224 | Per kart id. See section 4. |
| `kartphysicalparam.bin` | 7600 (A), 7400 (Menu), 5328 (dev copy) | 50 entries of 152 B (A) or 148 B (Menu). Speed, acceleration and weight values (fx20.12). The first 36 are indexed by kart id; the 14 extra entries are unidentified. |
| `kartAIparam.bin` | 5760 = 36 x 160 | CPU driving constants per kart. |
| `charphysicalparam.bin` | 104 = 13 x 8 | Two fx20.12 per character id. |
| `kartcrsmat.bin` | 432 | Per-surface constants. Unknown. |
| `kartmenuparam.bin` | 600 = 50 x 12 | Six u16 per entry, values 16..100. Menu stat bars. [I] |
| `charmenuparam.bin` | 26 = 13 x u16 | Per character, values 16..56. Menu stat bar. [I] |
| `staff_ghost_time.bin` | 160 | Not character data. |

`data/Main2D.carc/kart_appear.ktbl` (487 B). [C structure, **I** meaning of values]
- Header: magic "NKKT", u8 rows = 37, u8 columns = 13, then 37 x 13 bytes.
- Row is the kart id, column the character id. Values are 0..4.
- The save code (`save_is_entry_unlocked` 0x0205FC98) tests `value != 0 && value <= unlock_limit` with a 13-byte row stride, so the value is an unlock tier: 1 is default, 2 and 3 are shared or later karts, 4 is highest or locked.
- HH column is 0 for karts 0..35. Row 36 allows only HH (value 1).
- This is the kart availability and unlock matrix. The unlock limit comes from the save profile via 0x02056D98. [I]

### Per-character files
Paths relative to the archive root.

| Role | File | Notes |
|---|---|---|
| Race driver (detailed) | `character/<dir>/P_XX.nsbmd` + `.nsbtx` | Rigged, 5..8 bones. |
| Driver clips | `P_XX_{drive,spin,win,lose}.nsbca` | Table at 0x021551B0. |
| Face pattern anim | `character/common/P_faceanim.nsbtp` | Shared by all characters. 1 clip, 2 frames. |
| Static low-detail driver | `character/<dir>/E_XX.nsbmd/.nsbtx` | 1 node. Used when the "detailed" flag is 0. [C] |
| Kart (race and menu) | `kart/<dir>/kart_XX_{a,b,c}.nsbmd/.nsbtx` | Static, 1 node. |
| Tires | `kart/tire/kart_tire_{S,M,L}.nsbmd/.nsbtx` | Shared by all karts. |
| Shadow | `kart/shadow/sh_XX_{a,b,c}.nsbmd` | Untextured. Path format `%s%sshadow/sh_%s.nsbmd`. |
| Emblem | `emblem/XX_emblem.nsbtx`, `Team{Red,Blue}_emblem.nsbtx` | 32x32 texture "emblem" with palette "emblem_pl". |
| Shy Guy colour | `character/heyho/HH_color.NCLR` | |
| Select screen | `CharacterKartSelect*.carc` | Sprite sheets (NCGR/NCER/NCLR), localised name sheets, `kart_select.bmg`. |
| HUD / results icons | `Main2D_us.carc/player_character_{L,M}.*`, `Main2D.carc/player_character_*_shyguy_o.NCLR`, `Scene/*/Kart_icon_*`, `Scene/Race.carc/race_own_character_s_o.NCLR`, `Scene/Result*.carc` | Sprite sheets with palettes. Cell indexes are by character id. [I] |
| Voices | `data/Sound/sound_data.sdat` | Selected by character id (`sound_get_driver_group`, `sound_get_driver_voice`). Not traced further. [I] |

## 3. Driver skeleton (P_ models)

All driver NSBMDs have model scale 1.0 (posScale 4096). Every bone has scale 1.

### Retail skeletons, measured [C]

| Char | Bones (index: name, parent) | Meshes |
|---|---|---|
| MR, DK, KO, KP, YS, LG, DS | 7: 0 root, 1 body(0), 2 LarmA(1), 3 LarmB(2), 4 RarmA(1), 5 RarmB(4), 6 head(1) | 2 |
| WL | Same 7, but bone 0 is named `P_WL_root` | 2 |
| PC | The 7 plus 7 hair(6) | 2 |
| KA | 0 root, 1 body, 2 LarmA, 3 LarmB, 4 RarmA, 5 RarmB, 6 jaw(1), 7 head(6) | 1 (8 matrices) |
| RB | 6: 0 root, 1 body, 2 chest(1), 3 LarmA(2), 4 RarmA(2), 5 head(1). No forearm bones. | 2 |
| HH | 5: 0 `P_HH_root`, 1 body, 2 LarmA, 3 RarmA, 4 head | 1 |

(Two of the 7-bone characters, for example WR and DK, have slightly different bind translations; the table lists topology only.)

- Only the upper body exists. There are no leg or pelvis bones. Models are cut off at the waist and the kart tub hides the rest. This is the "legs get cut off" behaviour.
- Bind pose, Mario, for reference [C]: root is identity; `body` has a local rotation of 90 degrees about Z; shoulders sit at local (0.203, -0.094, 0) and (0.203, +0.094, 0) relative to `body`; forearms are 0.172 along local X from the upper arm; head is at 0.281 along local X. Bones run along +X.
- Other characters differ in translation (for example PC shoulders at 0.278; RB `body` at y 0.132).
- **Animations are keyed by bone index, not name.** The NSBCA joint list is bone 0..N-1. [C]
  - Bone 0 is static in all clips. Bone 1 carries rotation only.
  - Bones 2..N-1 carry translation and rotation tracks. The translation tracks equal the bind translation on every frame (constant) for all characters except R.O.B., where 1 to 2 joints change position.
  - Rotations vary on 6/7 bones for the standard 7-bone rig.
  - A clip therefore replaces local rotation absolutely. The new skeleton must keep the same bone count, order, parents, local translations **and the same bone-local axis convention** (the bind rotation of each bone) as the donor.
- A rig mismatch (count, order, or axes) makes arms and head twist wrongly; a clip addressing a missing bone index is probably skipped or corrupts the pose. [I]

### Skinning [C]
- Rigid, one bone per vertex. No weight blending in retail (every vertex uses joint[0] only).
- The mesh is split into polygon batches, each drawn under its bone through the matrix stack (5..8 matrices).
- Mesh layout (MR): `polygon1` / material `P_face` (face quad, about 100 verts) is rigid on `head`; `polygon0` / material `P_main` (body, about 200 verts) has vertices spread over bones 0..6.

### Materials and textures [C]
- Names looked up at run time: `P_main`, `P_face`; for E-models `E_main`, `E_face`; for karts `kart_body`, `kart_emblem`, `kart_tire`; the emblem texture "emblem" with palette "emblem_pl". Strings at 0x02154FF8..0x021550B8; consumers `menu_apply_preview_material`, `menu_upload_preview_palettes`, `menu_load_driver_palette`, `kart_upload_palette`, `kart_upload_face_textures`. Lookup is by name in the texture/palette dictionaries.
- Driver textures are all DS format 3 (16 colours, 4 bpp) with one 16-colour palette each:
  - `P_main`: 1024..2048 texels (32x32, 64x32, 32x64).
  - `P_face.1` and `P_face.2`: 256..1024 texels (16x16, 16x32, 32x32). Two eye states.
  - KA has no face material (1 material). HH has only `P_HH_main` 32x32.
- Face animation: `P_faceanim.nsbtp` (a PAT0 clip, 2 frames) switches the `P_face` material between `.1` (open) and `.2` (closed). The game selects closed for spin and lose. [C]

### Driver clips [C]

| Index | Name | Frames (range) | What it animates |
|---|---|---|---|
| 0 | drive | 29 | Steering pose. Frame equals steering offset plus centre (not a timer); about 14 is centred. Blends at 410/4096 per tick. |
| 1 | spin | 20..60 | Hit/spin reaction, played immediately. |
| 2 | win | 50..174 | Goal, ranks 0..2 per finish table at 0x021551DC. |
| 3 | lose | 59..260 | Goal, losing classification. |

Clip name table at 0x021551B0 (12-byte records). File names follow `%s%s_%s.nsbca`. `kart_setup_driver_animations` (0x0207954C) loads these four plus the shared face clip when its third argument (the "detailed" flag) is non-zero; with 0 it loads the static `E_` model. [C]

## 4. Kart model and attachment

### Kart NSBMD [C]
- One node (`world_root`, or the model's own name in some). Identity transform, 1 matrix.
- Exactly 3 materials / 3 polygons:
  1. `kart_body` (texture `kart_XX_?`, 16 colours): main body.
  2. `kart_emblem` (texture "emblem" 32x32, 16 colours): small quad (4..22 verts), the emblem decal.
  3. `kart_tire` (texture `E_tire_{S,M,L}`, 32x32 format 5): baked 4-wheel low-poly mesh of about 88 verts, probably a low-detail wheel set. [I]
- There are no wheel nodes.

### Wheels [C]
- `kart_draw` (0x02063CC4) draws the body model, then each tire as a **separate model** (`kart_tire_{S,M,L}`), applying a Y rotation (`MTX_RotY33`) for steering and an X rotation (`MTX_RotX33`) for roll/spin, then the driver and the shadow.
- Wheels spin and steer from data, not from bones.

`kartoffsetdata.bin`, 224-byte entry per kart id:
- 16 B tire model name (`kart_tire_M/S/L`).
- 52 x s32 in fx20.12:
  - [0] tire scale (0.6 / 1.0 / 0.8 for M/S/L).
  - [1..12] four wheel centres (x,y,z): FL = (+x,+z), FR = (-x,+z), RL = (+x,-z), RR = (-x,-z).
  - [13..51] thirteen driver seat offsets (x,y,z), one per character id; typically x about 0, y 0.18..0.24, z -0.06..-0.21.
- Example (class M): front x ±4.7, y 1.55, z 6.05; rear x ±5.5, y 2.5, z -4.3. These are kart-object units; the ratio to NSBMD units is roughly 10..16 [I] (draw scale not traced; calibrate against a retail kart).

### Driver attachment [C/I]
- The driver (P_ or E_) is drawn at the kart origin plus the seat offset for (kart id, character id) from `kartoffsetdata.bin` [C]. There are no attachment nodes in either model [C]; driver bone 0 sits at the seat point and the body bone's 90-degree rotation maps the driver's axes onto the kart's [I].

### Shadow, emblem
- Shadow `sh_XX_{a,b,c}`: one node, one untextured material named `sh`, 26..82 verts. Extent about ±0.35 in x and -1.25..+0.31 in z. [C]
- Emblem replaced at run time with a 16-colour 32x32 texture; team emblems for Red/Blue. [C]

## 5. Budgets (measured on retail)

| Asset | Count | Verts min / avg / max | Tris min / avg / max | Nodes | Materials | Max file bytes |
|---|---|---|---|---|---|---|
| P_ driver (race/menu) | 12 | 302 / 399 / 536 | 162 / 200 / 237 | 6..8 | 1..2 | 11084 |
| P_HH | 1 | 257 | 145 | 5 | 1 | 5548 |
| E_ static driver | 13 | 149 / 268 / 385 | 71 / 121 / 171 | 1 | 1..2 | 7500 |
| Kart (race A/B) | 25 | 309 / 361 / 429 | 159 / 192 / 241 | 1 | 3 | 7324 |
| Kart (menu) | 25 | 309 / 353 / 485 | 159 / 186 / 247 | 1 | 3 | 7840 |
| Tire (race / menu) | 3 | 56 / 96 | 32 / 64 | 1 | 1 | 1348 / 2028 |
| Shadow (race) | 25 | 26 / 55 / 82 | 20 / 44 / 60 | 1 | 1 | 904 |

Texture constraints: driver has 1 body texture (16 colours, at most 2048 texels) plus 2 face textures (16 colours, at most 1024 texels). Kart body is 512..2048 texels (16x32, 32x32, 64x32), emblem 32x32, tire 32x32 format 5. An archive size budget exists (`fs_set_kart_archive_budgets`, 0x020474E4); keep new files near retail size. [C name, I limit]

## 6. What "rigging an unrigged character" must produce [I, grounded in the above]

Goal: reuse a donor character's clips unchanged.
1. Pick a donor skeleton (default MR, 7 bones). Copy it verbatim: bone names, order, parents, local TRS and bone-local axes. Do not retarget the clip data.
2. Fit the imported mesh to the donor bind pose: torso on `body`, shoulders about ±0.094 from centre, upper arm 0.2 plus forearm 0.17, head about 0.28 above the body bone.
3. Cut the model at the waist. The kart hides everything below and nothing animates there.
4. Assign each vertex to exactly one bone (rigid). Suggested split: head to `head`, arms to A/B by elbow position, the rest to `body`. Keep the face region as its own mesh under `head`.
5. Split into at most 2 meshes: face quad (material `P_face`, textures `.1` and `.2`) and body (`P_main`).
6. Quantise textures to 16 colours (format 3): body at most 64x32 (or 32x64), faces 32x32 or smaller.
7. Write the NSBMD with rigid per-batch matrix assignment; matrix-stack count equals the bone count.
8. Reuse the donor's four NSBCA files (they contain only rotations over the same bone indices).

## 7. Tool recipe: glTF to a playable character

Replace the slot of an existing character (see open question 6 for adding a 14th).

| Output | Format and constraint | Auto or user |
|---|---|---|
| `character/<dir>/P_XX.nsbmd` + `.nsbtx` in KartModelSub and KartModelMenu | Donor skeleton verbatim, 2 meshes, rigid skin, at most 536 verts / 237 tris, 16-colour textures | Skeleton, material names, polygon split: auto. Scale and placement: auto with user nudge. Vertex-to-bone assignment: auto by distance, user can paint. |
| `P_XX_{drive,spin,win,lose}.nsbca` (Sub), `P_XX_win.nsbca` (Menu) | Copy of donor clips | Auto |
| `E_XX.nsbmd/.nsbtx` (MainA or B) | 1 node, materials `E_main` and `E_face`, at most 385 verts, format 5 allowed | Auto: bake the rest pose into one static mesh |
| Seat offsets | `kartoffsetdata.bin`, 3 s32 in the character's slot of each kart entry | Auto (copy donor), user nudge |
| Icons | Select / HUD / result sprite cells for the character | Auto resize of one portrait; user confirms the crop |
| Voices | Shared sdat entries | Reuse donor |

Sub and Menu both hold a copy of `P_XX.nsbmd`; keep them identical. [I]

## 8. Tool recipe: glTF to a drivable kart

Replace in KartModelMainA or B (and the same file in KartModelMenu):
- `kart/<dir>/kart_XX_{b,c}.nsbmd` + `.nsbtx` (or `kart_MR_a` for the standard kart):
  - 1 node; 3 materials named `kart_body`, `kart_emblem`, `kart_tire`; 3 polygons.
  - Body at most about 430 verts / 240 tris; 16-colour texture, at most 64x32.
  - Emblem: small quad using the 32x32 `emblem` texture and `emblem_pl` palette (copy the donor's).
  - `kart_tire`: small baked wheel mesh with `E_tire_{S,M,L}` (copy the donor's).
- `kart/shadow/sh_XX_{b,c}.nsbmd`: untextured footprint, 26..82 verts. Auto-generate from the body's convex hull.
- `kartoffsetdata.bin` entry for the kart id: tire class S/M/L; four wheel centres and tire scale (auto: find the four lowest, widest clusters of the imported mesh, or the user marks them); 13 driver seat offsets (default copy from donor).
- `kartphysicalparam.bin`, `kartAIparam.bin`, `kartmenuparam.bin` rows: copy a donor kart's row; user may edit.
- `kart_appear.ktbl` row: which characters may pick it.

The kart model must **not** contain wheel nodes; the tire model is instanced at the 4 offset positions and steered and spun by the game. The user supplies wheel positions only if auto detection fails.

## 9. Open questions
1. Placement calibration (2026-11-04): use 16 kart-object units per NSBMD unit for picked wheel centres. The owned standard kart's baked wheel bounds give X/Z ratios about 15.87..16.14 (rear X exactly 16); differences include the low-detail tire shape. Drawing also reduces world translation by four bits before GX output. Seat offsets are already in model units and are not multiplied by 16. Recheck wheel centring visually after choosing a different tire size.
2. Meaning of the extra 14 entries of `kartphysicalparam.bin`, and the named weight classes 0..2.
3. Exact meaning of `kart_appear.ktbl` values 1..4, and where the unlock limit comes from.
4. Where the baked `kart_tire` mesh is drawn (menu or far LOD) and whether it can be removed.
5. Resolved for race voices (2026-11-04): SSAR/SEQARC index 2 contains 182 records = 13 characters x 14 events. Playback at 0x0210AF40 / 0x0210B198 indexes character*14+event. The owned ROM confirms each group uses bank 3+character. Copy the donor event records while retaining the destination bank id, and copy the donor BANK INFO record (file and wave references) into that destination bank slot. `sound_get_driver_group` at 0x0211127C is a result-classification helper, not a character voice routing table. Menu/other voice variations still need an audible boot check.
6. Adding a 14th character: tables are hard-sized to 13 (name table, offset table, ktbl columns, save stride 13, charphysical/charmenu params). It needs code patches or a plugin; replacing an existing slot (for example Shy Guy, id 12) is the safe route.
7. Safe geometry/VRAM limits beyond the retail maxima.

# MKDS (USA, AMCE): tracks, audio, items, text and cups - structure reference

Scope: structure only (paths, formats, tables, sizes, how things link). No game bytes, textures, strings or code are reproduced.
Method: ROM file listing, the Toolbox CLI/parsers, the decompressed ARM9 image (BLZ-decoded; RAM base 0x02000000, 0x021773B8 end), the lifted source/dictionary, and web sources.
Legend: **[C]** confirmed (seen in ROM or lifted source this session), **[I]** inferred (pattern/guess, needs a test), **[W]** from web sources.

Web sources: MKDS ARM9 Music Editor (github.com/LandonAndEmma/MKDS-ARM9-Music-Editor, based on Ermelber's tool; its `code.js` lists the music-slot byte offsets), Mario Kart Toolbox NKM/KCL definitions (cited in `crates/nitro-formats/src/nkm.rs`), marIOkart `mkds` docs (blayyyyyk.github.io/marIOkart: OBJI 0x3C bytes, object id u16 at +0x24, route id at +0x26), TCRF Mario Kart DS pages (NKM versions 0x1E/0x24/0x25 in unused courses). No web source documented the DS KCL attribute bits or object-id names; those were derived from the ROM (sections 2 and 2.4).

## 0. Key numbers at a glance

| Item | Value | Status |
|---|---|---|
| Course table | 55 entries x 8 bytes at RAM 0x02154228: `[name pointer, kind]`; id 0 is an empty placeholder, ids 1..54 are real. (The earlier `--list-courses` view at 0x0215422C read `[kind, name]` shifted by one entry; course ids are the position in the 0x02154228 layout) | [C] |
| Course file path format | `/data/Course/%s%s%s.carc` = name + variant suffix + `Tex` or empty (strings at 0x02165174) | [C] |
| Cup table | 8 cups x 4 x u32 course ids at 0x02154128 (flat position = cup*4+race, 0..31) | [C] |
| Music slot table | 4-byte records from 0x0215316C; byte 1 = SEQ id (index in SDAT SEQ list); 51 records to 0x02153238 | [C] (offsets cross-checked with the web editor) |
| Course id -> music record | branch jump table at 0x0202554C (code, one `b` per course id 0..53) inside function 0x0202553C | [C] |
| SDAT | `data/Sound/sound_data.sdat`, 3,321,568 B, 76 SEQ, 5 SEQARC (SSAR), 99 BANK, 104 WAVEARC, 10 PLAYER, 2 GROUP, 0 STRM, no symbol block | [C] |
| Sound heap | 716,800 B when sound mode == 2, else 50,176 B (function at 0x021087C4) | [C] |
| Course names text | `common.bmg` (US) message ids 45..76, indexed by flat cup position; cup names ids 37..44 | [C] |
| Retail geometry budget | model triangles 120 / 1362 avg / 2026 max; KCL prisms 120 / 986 / 1700 | [C] measured |

## 1. A course's files

Every course id (table in section 6.1) names two archives in `data/Course/`: `<name><variant>.carc` and `<name><variant>Tex.carc` (path format `/data/Course/%s%s%s.carc`; [C]). `<variant>` is `D` only for the five courses whose course-table `kind` is 1 (old_momo_64, old_luigi_gc, mansion_course, cross_course, beach_course); those have a second archive pair `<name>D` / `<name>DTex` [C: file names and the kind flag are both observed; that kind==1 selects the D pair is [I]]. 59 archive pairs exist (54 ids + 5 D variants). Name lookup is case-insensitive in practice: the table stores `mr_stage1`, the ROM file is `MR_stage1`.

### 1.1 `<name>.carc` (geometry/data archive; ROM size 18.8 KB min, 135.8 KB avg, 228.1 KB max)

| Member | Count over 59 | Required? | Notes |
|---|---|---|---|
| `course_model.nsbmd` | 59 | required (path literal at 0x02165404) | the visible track model. Textures come from `<name>Tex.carc/course_model.nsbtx`; 9 of 59 courses have no .nsbtx because textures are embedded in the nsbmd (TEX0 block) [C] |
| `course_collision.kcl` | 59 | required (string at 0x0216B8F4) | collision (section 2.3) |
| `course_map.nkm` | 59 | required (path literal at 0x021647D8) | course logic: start, checkpoints, routes, objects, cameras (section 2.1) |
| `course_model_V.nsbmd` (+ `_V.nsbtx` in Tex) | 50 (47 with nsbtx) | optional | sky/background model ("V" = vista); 556 B min, 2.2 KB avg, 17.3 KB max. Path literals 0x02165454 / 0x0216546C / 0x02165484 [C]. A course without it presumably draws no sky model [I] |
| `course_model.nsbta` / `.nsbtp` / `course_model_V.nsbta` | 26 / 5 / 16 | optional | texture-SRT animation (water/lava scroll) and texture-pattern animation; literals 0x02165418..0x02165440 [C] |
| `MapObj/*` | 367 files total | only for object ids that use course-local models (section 2.2) | per-course object models and animations (`.nsbmd .nsbca .nsbma .nsbtp .nsbta`); path `MapObj/%s` (0x0216B23C) |
| `MissionRun/mrNN_tool.nkm` | 65 files | optional for races | mission-mode variants of the map, path `/MissionRun/%s_tool.nkm` (0x021647E8); not needed for GP/VS/TT [I] |

### 1.2 `<name>Tex.carc` (textures and minimap; ROM size 27 KB min, 73 KB avg, 134 KB max)

| Member | Count | Required? | Notes |
|---|---|---|---|
| `course_model.nsbtx` | 50 | required unless textures are embedded in the nsbmd | 9.6 KB min / 57 KB avg / 130 KB max |
| `course_model_V.nsbtx` | 47 | with the sky model | |
| `Map2D/local.NCGR` + `local.NCLR` | 59 | required | course-specific minimap tiles/palette (8,256 B min .. 65,600 B max; 65,600 B looks like 256x256 8bpp [I]). Path literals 0x0216812C..0x021682A8 [C] |
| `Map2D/local2.NSCR` | 59 | required | minimap tile map (screen); `local3.NSCR` exists on 27 courses (second layer/variant) [I] |
| `Map2D/global.NCGR/NCLR`, `global1.NSCR`, `global2.NSCR` | 55 | 55 courses have it | shared-looking minimap layer (frame/background); the 4 missing are the mission/staff-roll pseudo-courses [I]. `global2.NCGR/NCLR` (44 courses) is a second image |

The minimap is 2D art (tiles + screen map), not the 3D model. The mapping from world coordinates to minimap pixels was not located (not an obvious NKM field): open question (section 8).

### 1.3 Loose per-course data outside the archives

| Path | Count | Purpose |
|---|---|---|
| `data/CupPicture/select_cup_{nitro,retro}NN_m_{b.NCGR,b.NCLR,picture.NSCR}` | 16 + 16 sets (96 files) | course preview picture on the cup course-select screen, one per cup course; path built as `/data/CupPicture/select_cup_` + `%s%s%02d%s` [C]. NN = 1..16 within nitro (flat cup pos 0..15) and retro (pos 16..31) [count C, order I]; NCGR 3,584 B, NCLR 552 B, NSCR 2,084 B each. These are course pictures, not cup icons |
| `data/Ghost/ghost_000..031.bin` | 32 x 4,096 B | one staff-ghost slot per cup course [I by count] |
| `data/Ghost/staff_ghost_time.bin` | 160 B (32 x 5 B) [I] | staff times per cup course |
| `Net/net_course_map.nkm` | not in ROM | a string at 0x02164800 suggests a Wi-Fi variant; no such member exists in the listing [I: unused] |

Battle courses (ids 41..47, `mini_*`), mission courses (`mr_stage1..4`), `Award`, `StaffRoll`, `StaffRollTrue` use the same archive-pair layout.

## 2. What the game needs from a new track

### 2.1 NKM (`course_map.nkm`)

Container: magic `NKMD`, u16 version, u16 header size, then u32 offsets (relative to the header end) to each section; each section = 4-char tag, u32 count (STAG has no count: exactly one 40-byte record), then records. Section order in all retail files: OBJI, PATH, POIT, STAG, KTPS, KTPJ, KTP2, KTPC, KTPM, CPOI, CPAT, IPOI, IPAT, EPOI, EPAT, AREA, CAME; battle files add MEPO, MEPA. Versions seen: 37 (49 files), 30 (4), 34 (3), 32 (2), 36 (1). KTPJ is 32 B from version 32 else 28 B; IPOI is 20 B from version 34 else 16 B [C: `nkm.rs` strides + measured files].

Record sizes (bytes): OBJI 60, PATH 4, POIT 20, STAG 40, KTPS/KTP2/KTPC/KTPM 28, KTPJ 28/32, CPOI 36, CPAT/IPAT/EPAT 12, IPOI 16/20, EPOI/MEPO 24, MEPA 20, AREA 72, CAME 76 [C].

Counts over the **32 cup race courses** (measured):

| Section | Meaning | min | avg | max | courses with 0 |
|---|---|---|---|---|---|
| KTPS | race start position (single record; the 8-kart grid is built by code) | 1 | 1.0 | 1 | 0 |
| KTPJ | respawn points (Lakitu drop; link to nearest enemy/item route point [I]) | 2 | 7.5 | 16 | 0 |
| KTP2 | second start position | 1 | 1.0 | 1 | 0 |
| KTPC | cannon destinations | 0 | 0.1 | 1 | 30 |
| KTPM | mission start | 0 | 0 | 0 | 32 |
| CPOI | checkpoints (gates: two XZ end points, sin/cos, distance, section links, key id [I: field list]) | 16 | 37.4 | 78 | 0 |
| CPAT | checkpoint paths (groups with next/prev links) | 1 | 1.3 | 4 | 0 |
| IPOI / IPAT | item route points / paths | 14 / 1 | 66.7 / 6.4 | 128 / 38 | 0 |
| EPOI / EPAT | CPU (enemy) route points / paths | 34 / 1 | 77.6 / 5.2 | 174 / 18 | 0 |
| POIT / PATH | object route points / paths (moving objects; speed in setting 1) | 30 / 5 | 95.9 / 11.2 | 298 / 20 | 0 |
| OBJI | placed objects | 11 | 31.6 | 56 | 0 |
| AREA | zones (camera/effect/boundary triggers) | 5 | 14.2 | 23 | 0 |
| CAME | cameras (intro, goal, zone cameras) | 12 | 20.0 | 26 | 0 |
| STAG | one record: +0 course id, +2 laps (race) or match seconds (battle) [C: dictionary] | 1 | 1 | 1 | 0 |

Battle courses (7): KTPS 8 and KTPJ 8 (one per player), MEPO 32..201 / MEPA 11..61 (battle CPU routes), no CPOI/IPOI/EPOI, OBJI 22..34, CAME 0..8.

Practical minimum for a drivable GP track (every retail race course has all of these): KTPS 1, KTP2 1, KTPJ >= 2, CPOI >= 16 gates in a closed chain via CPAT, EPOI/EPAT >= 1 closed route, IPOI/IPAT >= 1 route, CAME >= 12 (intro/goal/etc. [I]), AREA >= 5 (retail minimum; whether fewer works is untested), STAG with the right id and lap count. Anything below the retail minimum is [I] untested.

Linking: CPOI chains by section links into CPAT groups; KTPJ respawn points are used by `kart_respawn_at_checkpoint` [C: function names]; OBJI route id (+0x26) selects a PATH (0xFFFF = none) [W]; the lap count lives in STAG +2 [C].

### 2.2 Objects (OBJI: object id u16 at +0x24, route id at +0x26, four u32 settings, one u32 time-trial visibility flag)

96 distinct object ids appear in retail NKMs. Families by id range, derived by correlating each id with the `MapObj` files present in every course that uses it [I]:

| Id range | Use | Files needed |
|---|---|---|
| 0x65 (101) | item box: 639 placements in 50 courses; model `itembox*` is in global `data/Main/MapObj.carc`, plus `data/MainRace.carc/MapObj/{box,question,obj_shadow}` | none per course |
| 0x66 (102), 0x08 (8), 0x0A (10), 0xC9 (201), 0xCA (202) | used by many courses with no course-local file in common | none per course (global); semantics unknown, so only 0x65 is a safe, understood palette entry |
| 0x67 (103) | wooden box | `woodbox1.nsbmd` in the course |
| 0x69 / 0x6B (105/107) | battle-stage shine/balloon objects | `shine16`, `balloon` models (battle only) |
| 0x01..0x0F | misc hazards/effects each tied to specific course files (water effect `water_efct.*` for 5 and 14, bats for 9/15) | course-local |
| 0xC9..0xD2 (201..210) | moving gear/pendulum/bridge/rotary objects (Tick-Tock, Bowser) | course-local models |
| 0x12D..0x158 (301..344) | scenery by id: trees, water planes, houses; the id fixes which `MapObj/<model>` is loaded | the specific model(s); copy from the donor course |
| 0x191..0x1B5 (401..437) | enemies/hazards: 401 goomba-type, 405/410/412 buses/cars, 409 mole, 411 cheep, 422 piranha, 424/425 pinball bumpers | the model plus its `.nsbtp` (e.g. `kuribo.*`, `bus_a`/`car_a`, `choropu.*`, `pukupuku`, `ob_pakkun_sf.*`, `bound`/`flipper`) |

Rule for the Toolbox palette: an object id is only safe on another course if its `MapObj` files are copied into that course's carc. The map-object class handler table is at RAM 0x0216B94C and the own-reaction table at 0x0216B9AC [C: dictionary]; the table mapping object id to class/model files was not located (open question).

### 2.3 KCL collision (`course_collision.kcl`)

Header 60 B: u32 positions offset, normals offset, prism offset minus 16, octree offset; thickness; minimum corner (3 fixed-point); three masks, block shift at +44; then arrays: positions (12 B), normals (6 B, s16/4096 x3), prisms (16 B: fixed height, u16 position index, u16 face-normal index, three u16 edge-normal indices, u16 attribute), then the octree [C: `kcl.rs`]. Retail: 59 files, 10 KB..288 KB; prisms 120 min / 986 avg / 1,700 max [C measured].

Attribute u16 decode [C: the kart surface dispatcher at 0x01FF9E5C computes TYPE = (attr & 0x1F00) >> 8 and VARIANT = (attr & 0xE0) >> 5]:

| Bits | Field | Observed |
|---|---|---|
| 8..12 | surface type; valid 0..21 (22-entry handler table at RAM 0x02154D8C) | 0..21 on driven prisms; 22 on a few trigger prisms |
| 5..7 | variant 0..7 (effect/sound/dust variant) | all values |
| 13..15 | class flags | 0x8000 on floors (95-100% upward normals); 0x4000 on walls (types 8, 14, 16, 21; type 9 with 0x6000); 0x2000 alone on trigger-like prisms (types 4, 11, 22) [I]; none on types 11/15 in a few courses |
| 0..4 | minor flags (even values only: 0,2,4,6,8,10,12,14,16,20,24) | meaning unknown [I]; types 13 and 14 almost always carry 0x10 |

Handlers per type (names are dictionary names of the functions the handler calls) [C]; meanings [I]:

| Type(s) | Handler behaviour | Likely meaning / retail evidence |
|---|---|---|
| 0 | default effect dispatch | normal road (15,895 prisms, 55 courses) |
| 1, 6, 10, 17, 20 | own effect-dispatch handlers | alternative ground: sand/dirt/ice/snow/carpet variants (1 in desert/beach/snow; 6 mansion; 17 only bank_course; 20 rainbow/mini stages) |
| 2, 3, 5 | `kart_update_wading` + effects | shallow water / mud / deep sand where the kart wades (type 3 on 25 courses) |
| 4, 8, 9, 11, 13, 14, 16, 21 | shared default handler (0x02067AD4) | walls use 8/9/14/16/21 with the 0x4000 flag (8 = standard wall, 47 courses) |
| 7, 12 | `kart_apply_boost` (7) / sound + effect clear (12) | speed-affecting surfaces |
| 18 | `kart_start_terrain_boost` | boost pad (stadium, wario, donkey courses) |
| 19 | boost-ramp handler; returns surface kind 11, sets bit 0x40000000 | boost ramp / jump (only Rainbow Road) |
| 15 | `kart_respawn_at_checkpoint` and `kart_cannon_enter` | out-of-bounds/fall-off and cannon trigger (variant selects which) |

KartStats holds grip and speed arrays of 12 surface kinds at +0x38 / +0x68; the handler returns the kind that indexes them [C]. No separate lava type was found: fall-off/lava is most likely type 15 plus variant; water splash is 2/3/5 [I]. Slope handling is authored, not computed: walls are marked 0x4000 in the file [I]. For a generator: floor = 0x8000 | (type<<8) | (variant<<5), road = 0x8000; wall = 0x4800 (second most common attribute, 6,088 prisms) or 0x6900 (wall with the extra 0x2000 flag, on 43 courses, probably also blocks items [I]).

### 2.4 Budgets measured on the 59 retail courses (NSBMD display lists parsed)

| Measure | min | avg | max |
|---|---|---|---|
| triangles in `course_model.nsbmd` | 120 (test_circle) | 1,362 | 2,026 (rainbow_course) |
| vertices submitted | 206 | 2,346 | 3,734 |
| polygon/material shapes | 3 | 14 | 28 |
| nsbmd bytes | 12,612 | 39,155 | 124,952 |
| nsbtx bytes (50 courses) | 9,636 | 57,176 | 129,780 |
| KCL prisms | 120 | 986 | 1,700 |
| KCL bytes | 10,152 | 115,420 | 287,602 |
| NKM bytes | 508 | 9,343 | 21,960 |
| minimap local.NCGR bytes | 8,256 | 57,890 | 65,600 |

The DS geometry engine renders about 2,048 polygons per frame, shared with karts and objects, so a track should stay under about 2,000 triangles (quads count as one polygon, so retail polygon counts are lower than these triangle counts). Texture VRAM is 512 KB total, shared with karts/objects/sky; keep course textures at or below the retail maximum of about 130 KB [I].

### 2.5 Most common reasons a custom track crashes or misbehaves [I unless noted]

1. A missing or zero-count section the code indexes unconditionally (CPOI, EPOI, IPOI, KTPS, KTPJ, CAME, AREA); no retail race course has zero of any of them.
2. Checkpoint chain not closed or CPAT links wrong: laps never count and the race never ends.
3. STAG course id not matching the slot being replaced, or lap count 0.
4. KCL problems: prism index outside the position/normal arrays or octree not covering the track (the Toolbox parser rejects both [C]); no 0x8000 floor under the start so karts fall through.
5. An object id whose `MapObj` files are not in the carc.
6. Model/texture over budget (more than about 2,000 polygons, unsupported texture formats), or `.nsbtx` missing while the nsbmd has no embedded textures.
7. Missing `Map2D/local.*` minimap files in the Tex archive (always present in retail).
8. Music: a SEQ whose bank/wave archive is not loadable or is too large for the sound heap (section 3).
9. Course `kind` 1 (D variant) without a `D` archive pair, or the reverse.

## 3. Music and sound effects

### 3.1 SDAT layout (`data/Sound/sound_data.sdat`, 3,321,568 B) [C: parsed]

Header `SDAT`; blocks SYMB (absent: both offsets 0, no names), INFO at 0x40 (0xFB0 B), FAT at 0xFF0 (284 entries), FILE at 0x21BC (3,312,932 B payload, 284 files). INFO record lists:

| List | Count | Notes |
|---|---|---|
| SEQ (SSEQ) | 76 | record: file id u32, bank id u16, volume u8, channel priority 64, player priority 64, player id u8. Sizes 188 B..21,476 B. Sequence index = the id used by the code/ARM9 table |
| SEQARC (SSAR) | 5 | sound-effect archives: 596 / 346 / 182 / 24 / 8 sequences (file sizes 26,796 / 13,768 / 4,460 / 596 / 868 B); per-entry bank ids are the SFX banks |
| BANK (SBNK) | 99 | each lists up to 4 wave archives; 188..1,364 B |
| WAVEARC (SWAR) | 104 | 60 B..326,168 B; 0..90 waves each |
| PLAYER | 10 | max sequences 1..3 per player |
| GROUP | 2 | only 2 groups exist, so per-song loading is by sequence (SEQ -> bank -> wave archives), not by group [I] |
| STRM / STRM_PLAYER | 0 / 0 | no streamed audio; every sound is a sequenced instrument or SWAV |

Wave data: all 864 waves are ADPCM (SWAV type 2); 421 loop, 443 do not. Rates seen 11,025 .. 22,050 Hz and a few 16,000 Hz. Wave header: type u8, loop flag u8, sample rate u16, time u16, loop start u16 (in 32-bit words, includes the 4-byte ADPCM header so minimum 1), length u32 (words) [C].

### 3.2 How a course picks its music [C unless marked]

1. ARM9 function `0x0202553C` converts a course id (0..53) into a "music record index" using a 54-entry branch table at RAM 0x0202554C (each entry is a `b` to a `mov r12,#idx`; ids 0, 2..8 and 21 have no entry, index 0x36 = none). Table below, column "music slot idx".
2. The index addresses the record table that starts at RAM 0x0215316C: 4 bytes per record, 51 records (to 0x02153238). Record layout: byte 0 = 2 for menu/scene records or 0xFF for race/battle, **byte 1 = SEQ id** (the Ermelber/LandonAndEmma editor edits exactly this byte; its offsets, file position 0x153171 etc., equal RAM address - 0x02000000 + 1 in the decompressed ARM9 and match what is listed here [C]), byte 2 = an auxiliary id 16..48 or 0xFF (meaning unknown [I]: values repeat across courses, e.g. 26 on 10 courses, so it is not a per-course second song; possibly a jingle/ambience or wave-set selector), byte 3 = 0xFF (0x22 on battle/boss records).
3. The same record table also holds menu and scene music: game intro, single/multi/Wi-Fi/records menus, the five Grand Prix flyover records, battle flyover, boss intro, GP results, credits (two records), reward, battle, boss battle [C names from the editor + values].
4. Final lap: the record has only one SEQ id. The code has a "mood" selector (`sound_get_race_music_mood` at 0x02110E58 returns 0 normal / 1 / 2 final lap), `sound_update_final_lap_music` (sound_code, counts frames past 130/145 then sends a sequence id) and `sound_play_final_lap_fanfare` (0x02106AB8). So final lap is handled by code (a fanfare plus a tempo/sequence-variable change on the same SSEQ, [I]); a modder gets it for free from the one SEQ slot [C: there is no second slot to fill]. Special courses (ids 20, 25, 34 in the mood code) have hard-coded alternate sequence ids in code: those are code patches, not table edits [C for the code path, I for the effect].
5. Loading: the chosen SEQ's bank and wave archives are loaded into the sound heap (716,800 B when sound mode is 2, otherwise 50,176 B; function 0x021087C4) [C for heap size, I for the mode-2 = race assumption]. A scene-level load list for non-race scenes exists at RAM 0x0216CBB8 (6 records x 24 bytes, 255-terminated lists) [C values, I meaning].

Retail course songs: SEQ 15..45 plus 74 (Mario Circuit / Figure-8 share SEQ 74), each with its own bank (49..89) and its own wave archive, plus the shared common wave archive 80 (83,660 B, 38 waves) that nearly every bank also lists. Per-song own wave data: max 64,408 B (SEQ 69/70, wave archive 95, 21 waves), typical 15-45 KB; SEQ files 2.8-21.5 KB. Biggest total (SEQ + bank + waves incl. archive 80) is about 148 KB.

### 3.3 Course -> music slot table

Columns: course id (position in the 0x02154228 table), internal name, course `kind`, music record index, SEQ id (byte 1), aux (byte 2), SEQ size, bank id. "-" = no record (unused/test ids; 54 has no branch entry).

| course id | internal name (carc stem) | kind | music slot idx | SEQ id (byte1) | aux (byte2) | SEQ size B | bank |
|---|---|---|---|---|---|---|---|
| 0 | (empty) | 2 | - | - | - | - | - |
| 1 | old_yoshi_gc | 0 | 9 | 33 | 26 | 12020 | 50 |
| 2 | old_mario_gc | 0 | - | - | - | - | - |
| 3 | luigi_course | 0 | - | - | - | - | - |
| 4 | dokan_course | 0 | - | - | - | - | - |
| 5 | test1_course | 0 | - | - | - | - | - |
| 6 | donkey_course | 0 | - | - | - | - | - |
| 7 | wario_course | 0 | - | - | - | - | - |
| 8 | nokonoko_course | 0 | - | - | - | - | - |
| 9 | old_baby_gc | 0 | 12 | 34 | 255 | 14680 | 56 |
| 10 | old_mario_sfc | 0 | 21 | 22 | 26 | 3896 | 69 |
| 11 | old_momo_64 | 1 | 13 | 30 | 31 | 9016 | 57 |
| 12 | old_koopa_agb | 0 | 36 | 27 | 255 | 6132 | 74 |
| 13 | old_peach_agb | 0 | 25 | 26 | 255 | 6960 | 73 |
| 14 | old_luigi_gc | 1 | 8 | 33 | 29 | 12020 | 50 |
| 15 | old_noko_sfc | 0 | 22 | 23 | 18 | 4932 | 70 |
| 16 | old_frappe_64 | 0 | 14 | 31 | 24 | 10404 | 58 |
| 17 | clock_course | 0 | 33 | 39 | 25 | 5052 | 83 |
| 18 | mansion_course | 1 | 20 | 21 | 19 | 6384 | 68 |
| 19 | airship_course | 0 | 16 | 18 | 27 | 8924 | 62 |
| 20 | cross_course | 1 | 7 | 74 | 16 | 11048 | 49 |
| 21 | test_circle | 0 | - | - | - | - | - |
| 22 | bank_course | 0 | 11 | 16 | 17 | 7688 | 55 |
| 23 | old_hyudoro_64 | 0 | 29 | 32 | 32 | 11540 | 79 |
| 24 | ridge_course | 0 | 27 | 36 | 23 | 9884 | 77 |
| 25 | mario_course | 0 | 34 | 74 | 26 | 11048 | 49 |
| 26 | garden_course | 0 | 19 | 20 | 29 | 9232 | 67 |
| 27 | desert_course | 0 | 31 | 38 | 20 | 9108 | 82 |
| 28 | town_course | 0 | 15 | 17 | 21 | 14608 | 59 |
| 29 | rainbow_course | 0 | 35 | 41 | 48 | 12412 | 85 |
| 30 | snow_course | 0 | 30 | 37 | 24 | 8656 | 81 |
| 31 | beach_course | 1 | 10 | 15 | 18 | 11504 | 51 |
| 32 | koopa_course | 0 | 37 | 40 | 30 | 7148 | 84 |
| 33 | pinball_course | 0 | 32 | 19 | 22 | 11160 | 65 |
| 34 | stadium_course | 0 | 17 | 19 | 28 | 11160 | 65 |
| 35 | old_donut_sfc | 0 | 23 | 24 | 26 | 3844 | 71 |
| 36 | old_choco_64 | 0 | 28 | 29 | 24 | 9964 | 78 |
| 37 | old_luigi_agb | 0 | 26 | 26 | 33 | 6960 | 73 |
| 38 | old_kinoko_gc | 0 | 18 | 35 | 23 | 8040 | 66 |
| 39 | old_choco_sfc | 0 | 24 | 25 | 26 | 3796 | 72 |
| 40 | old_sky_agb | 0 | 38 | 28 | 255 | 5508 | 75 |
| 41 | mini_block_course | 0 | 39 | 43 | 43 | 7976 | 89 |
| 42 | mini_block_64 | 0 | 39 | 43 | 43 | 7976 | 89 |
| 43 | mini_dokan_gc | 0 | 39 | 43 | 43 | 7976 | 89 |
| 44 | mini_stage1 | 0 | 39 | 43 | 43 | 7976 | 89 |
| 45 | mini_stage2 | 0 | 39 | 43 | 43 | 7976 | 89 |
| 46 | mini_stage3 | 0 | 39 | 43 | 43 | 7976 | 89 |
| 47 | mini_stage4 | 0 | 39 | 43 | 43 | 7976 | 89 |
| 48 | mr_stage1 | 0 | 40 | 45 | 255 | 8196 | 88 |
| 49 | mr_stage2 | 0 | 40 | 45 | 255 | 8196 | 88 |
| 50 | mr_stage3 | 0 | 40 | 45 | 255 | 8196 | 88 |
| 51 | mr_stage4 | 0 | 40 | 45 | 255 | 8196 | 88 |
| 52 | Award | 0 | 42 | 66 | 255 | 6256 | 94 |
| 53 | StaffRoll | 0 | 51 | - | - | - | - |
| 54 | StaffRollTrue | 0 | - | - | - | - | - |

Shared songs: ids 20 and 25 (cross_course, mario_course) both use SEQ 74. Menu/scene SEQ ids: intro 75, single-player menu 1, multiplayer 2, Wi-Fi 3, records 6, GP flyovers 8/9/10, battle flyover 13, boss intro 14, battle 43, boss battle 45, reward 63, GP results 66, credits 69/70 [C].

### 3.4 How to give a track its own song

Replace or add [I: neither was test-booted]:

| Approach | What changes | Notes |
|---|---|---|
| Replace | pick a retail SEQ slot, overwrite its SSEQ file in the SDAT FILE block and update the FAT sizes/offsets (SDAT must be rebuilt: all later offsets shift); if the new song needs new instruments, overwrite its bank (SBNK) and wave archive (SWAR) or point the SEQ INFO record at a new bank | smallest change; every course using that SEQ changes too (e.g. SEQ 74 is shared by two courses) |
| Add | append a new SSEQ (and SBNK/SWAR) to FILE, extend FAT, add an INFO SEQ record (index 76+), then write that index into byte 1 of the course's music record (a u8, so up to 255 [C]). Count lists in INFO and block sizes must be rewritten | gives each track a unique song; the record table is in ARM9, so this is an arm9 byte patch, no code change |

New songs must be authored as SSEQ (MIDI-like event stream with note/track/tempo/loop commands). A MIDI file maps well (tracks -> SSEQ tracks, program changes -> bank patches, tempo and loop markers). Instruments must exist in the bank: either reuse a retail bank + wave archive (any GM-like sound already present in wave archive 80 or in the original song's archive) or build a new SWAR from the user's samples.

### 3.5 Looping sample as a song, and realistic size

Mechanism [I]: a one-note SSEQ that plays one long looping ADPCM SWAV from a custom bank (SBNK with one region/note-def of type PCM/ADPCM pointing at the wave, wave header loop flag = 1 with loop start/length). All retail instrument waves are ADPCM at 11-22 kHz, and 421 of 864 retail waves loop, so the engine path is exercised.

Size limits [I from the numbers above]:

* ADPCM is 4 bits/sample, so bytes per second = rate/2: 11 KB/s at 22,050 Hz, 8 KB/s at 16,000 Hz, about 16 KB/s at the hardware maximum (~32 kHz).
* Retail songs keep their own wave data to <= 64 KB (about 6 s at 22 kHz). The sound heap is 716,800 B in mode 2 and also holds the common archive (83.7 KB), the SFX sequence archives and banks, character voice archives and the song itself, so a safe target is **a total of about 150-250 KB for the song's own wave archive** (about 14-22 s of mono at 22 kHz, or 9-15 s at 32 kHz); above ~300 KB is likely to fail to load [I, untested].
* Stereo needs two waves (left and right channels as separate hard-panned notes), which doubles the cost.
* SPU has 16 channels shared with kart sound effects; use at most 2 for a looped song.
* There is no streaming path (0 STRM entries), so the wave must be fully resident.

Mix (MIDI route): use the retail bank/wave archive 80 sounds to keep size near zero; or supply own instruments, budgeting the same 150-250 KB.

### 3.6 Sound effects

SFX are SSAR archives of tiny sequences (596/346/182/24/8 entries in 5 SEQARC files, 26.8 KB / 13.8 KB / 4.5 KB / 0.6 KB / 0.9 KB) using the banks listed per entry (SFX banks 0..44 range, wave archives 0..95 including the large common ones: archive 0 = 326,168 B/90 waves and archive 97 = 296,108 B/90 waves are the largest). To replace an effect: edit the wave inside the SFX wave archive (keep the same index, ADPCM, length at most the original unless the whole SWAR is re-laid out) or point the SSAR entry's bank at another one. The 12 game sound handles/7 slot lists and the sound id numbers used by the code (0x100..0x1D2 range seen in the race music code) are SEQARC-relative ids [I]. Character voices come from per-driver groups chosen by `sound_get_driver_group` (ids 63..68 range) [C name].

## 4. Items

| What | Where | Notes |
|---|---|---|
| Item 3D models (thrown/held/dropped) | `data/MainRace.carc/Item/it_*.nsbmd` (banana, bomb, gesso = blooper, kinoko = mushroom, kinoko_p, koura_g/r/w = shells, star, teresa = boo, thunder, f_box, killer) plus `koura_w.nsbmd`, `geso_sumi.nsbmd` (ink) and `it_bomb.nsbtp` | 0.9-4.7 KB each. Every file has MDL0 + TEX0 blocks, so the textures are embedded in the nsbmd (replace by re-exporting an nsbmd with its texture) [C]. Path format `Item/%s` (0x0216C5E4) |
| Item model for the bullet kart | `data/KartModelMainA.carc/character/common/KillerItem.{nsbmd,nsbtx,nsbca}` and `KillerItem_s.nsbmd` | separate texture file [C] |
| Item box model/animations | `data/Main/MapObj.carc/itembox.nsbmd/.nsbca`, `itembox_shadow.*`, `itembox_hahen.nsbmd` (break pieces); `data/MainRace.carc/MapObj/{box,question,obj_shadow}.nsbmd` | global; every course uses object id 0x65 |
| HUD item icons and roulette | `data/Scene/Race_us.carc/race_s_o.NCGR` (16,448 B sprite atlas: item icons, digits, "LAP"/"TIME" glyphs; `race_m_o.NCGR` is a similar-size main-screen atlas, content not inspected [I]) with palette `data/Scene/Race.carc/race_s_o.NCLR` and cell bank `data/Scene/Race.carc/race_s.NCER` (3,418 B) | one copy per language (`Race_us/es/fr/ge/it.carc`, each 16,448 B) [C]. The icon order inside the atlas was observed visually [C]; positions come from the cell bank (not decoded) [I] |
| Roulette / item selection screen | `data/Scene/Race.carc/choises_s_*` and `race_*` NSCR/NCGR | [I] |
| Which item a rank gets | `data/MainRace.carc/Item/ItemSlotList.dat` (1,391 B probability table by position/mode [I]) | gameplay table, not art |
| Item object logic | per-type 168-byte records at *0x0217BC14 (templates near 0x02159934), held-item state 528 B per kart | code-side; not data files [C: dictionary] |

## 5. Text (BMG)

BMG format: `MESGbmg1`, sections INF1 / DAT1 / MID1 (strings UTF-16 with escape tokens for icons/colors) [C: `bmg.rs`]. Each language has its own copy of the carc holding the BMGs; the US ROM carries `_us`, `_es`, `_fr`, `_ge`, `_it` (Main2D also has JA only for MBChild). Message counts (US):

| File (US path) | Msgs | Holds |
|---|---|---|
| `data/Main2D_us.carc/common.bmg` (6,336 B) | 140 | **cup names ids 37..44; course names ids 45..76 (flat cup position 0..31 + 45; confirmed by Mario Circuit at 55 = cup 2 race 3, Baby Park at 68 = cup 5 race 4); other common strings** including Grand Prix labels (35, 36) and generic course strings (96, 101) |
| `data/Scene/Menu_us.carc/menu.bmg` | 23 | menu labels |
| `.../Menu_us.carc/rule.bmg`, `MenuDL_us.carc/rule.bmg` | 24 | rule/option strings |
| `.../Menu_us.carc/mission.bmg` | 63 | mission mode strings |
| `data/Scene/Record_us.carc/record.bmg`, `ghost.bmg` | 13, 88 | records menu; ghost.bmg has 32 + 32 course names (ids 19..50 and 51..82, two variants) |
| `data/Scene/Ghost_us.carc/ghost.bmg`, `MenuDL_us.carc/ghost.bmg` | 88 | same ghost strings in the ghost scene and download scene |
| `data/Scene/WiFiMenu_us.carc/wifi.bmg` | 81 | Wi-Fi strings incl. course names (ids 49..80) |
| `data/Scene/Result_us.carc/secret.bmg` | 12 | unlock/secret messages mentioning cups |
| `data/Scene/Option_us.carc/option.bmg` 17; `StaffRoll_us.carc/staffRoll.bmg` 47; `Title_us.carc/title.bmg` 2; `WLMenu_us.carc/banner.bmg` 12; `Emblem_us.carc/emblem.bmg` 3; `CharacterKartSelect_us.carc/kart_select.bmg` 42; `Static2D.carc/MBChild_<lang>.bmg` 3 | | respective screens |

Cup names are **not text only**: the cup-select and cup-name headings are sprite art (localized): `data/Scene/Menu_<lang>.carc/common/select_cup_name_m.nce.{ncgr,ncer}` (20,528 B tiles), `select_cup_m.nce.*` (10,224 B) and `select_cup_s.nce.*` (15,152 B, touch screen). So renaming a cup needs the BMG message (id 37..44, used in text lists) **and** the sprite tile art in every language you ship; course names are text only (ids 45..76 in `common.bmg`, plus ghost/WiFi lists if those screens should match). Course name texts in the in-race pre-race banner: not located [open]. A course name for a replaced track: edit the message of the flat position it occupies in the cup table (section 6.3).

## 6. Cups

### 6.1 Course table (RAM 0x02154228, 55 entries x 8 B: name pointer, kind) [C]

Entry id = position. `kind`: 0 normal; 1 has a `D` archive pair (5 courses); 2 only on the empty id 0. Name strings are in the ARM9 data area at 0x02164900..0x02164CAF as plain file-stem strings. Second table right after it at 0x021543E0 (name pointer `/data/MainRace.carc`, -1, ...) is a different table: appending new entries needs relocation [C].

(See table in 3.3: first two columns.)

### 6.2 Cup table (RAM 0x02154128, 8 cups x 4 x u32 course id = 128 B) [C]

Used by `course_get_cup_entry` (0x02046828: cup = pos/4, race = pos%4), `course_find_cup_position` (0x0204684C, returns 0..31 or -1), `course_find_cup` (0x020467D0) and the cup-progress functions `race_setup_cup_progress` (0x02045914), `race_advance_cup_progress` (0x02045ED0), `race_advance_cup_course` (0x02046740), `race_reset_cup_progress` (0x020467A0). Literal-pool references to the table address in ARM9: 9 (0x0202E464, 0x0203B8D4, 0x02044938, 0x02046020, 0x02046518, 0x0204679C, 0x02046824, 0x02046848, 0x020468AC) [C]. The ARM9 course table itself has just 2 literal references (0x020484E8, 0x02048500). Overlays are compressed in the ROM, so overlay references were not scanned [open].

| cup | flat pos | course id (race1..4) | internal names | CupPicture files (flat pos+1) | common.bmg course-name msg ids (45+pos) |
|---|---|---|---|---|---|
| 0 | 0-3 | [20, 22, 31, 18] | cross_course, bank_course, beach_course, mansion_course | select_cup_nitro01_m_*, select_cup_nitro02_m_*, select_cup_nitro03_m_*, select_cup_nitro04_m_* | 45-48 |
| 1 | 4-7 | [27, 28, 33, 24] | desert_course, town_course, pinball_course, ridge_course | select_cup_nitro05_m_*, select_cup_nitro06_m_*, select_cup_nitro07_m_*, select_cup_nitro08_m_* | 49-52 |
| 2 | 8-11 | [30, 17, 25, 19] | snow_course, clock_course, mario_course, airship_course | select_cup_nitro09_m_*, select_cup_nitro10_m_*, select_cup_nitro11_m_*, select_cup_nitro12_m_* | 53-56 |
| 3 | 12-15 | [34, 26, 32, 29] | stadium_course, garden_course, koopa_course, rainbow_course | select_cup_nitro13_m_*, select_cup_nitro14_m_*, select_cup_nitro15_m_*, select_cup_nitro16_m_* | 57-60 |
| 4 | 16-19 | [10, 11, 13, 14] | old_mario_sfc, old_momo_64, old_peach_agb, old_luigi_gc | select_cup_retro01_m_*, select_cup_retro02_m_*, select_cup_retro03_m_*, select_cup_retro04_m_* | 61-64 |
| 5 | 20-23 | [35, 16, 12, 9] | old_donut_sfc, old_frappe_64, old_koopa_agb, old_baby_gc | select_cup_retro05_m_*, select_cup_retro06_m_*, select_cup_retro07_m_*, select_cup_retro08_m_* | 65-68 |
| 6 | 24-27 | [15, 36, 37, 38] | old_noko_sfc, old_choco_64, old_luigi_agb, old_kinoko_gc | select_cup_retro09_m_*, select_cup_retro10_m_*, select_cup_retro11_m_*, select_cup_retro12_m_* | 69-72 |
| 7 | 28-31 | [39, 23, 40, 1] | old_choco_sfc, old_hyudoro_64, old_sky_agb, old_yoshi_gc | select_cup_retro13_m_*, select_cup_retro14_m_*, select_cup_retro15_m_*, select_cup_retro16_m_* | 73-76 |

Cups 0..3 are the Nitro cups and 4..7 the Retro cups (picture prefix and message order agree). Cup names are message ids 37..44 and sprite art (section 5).

### 6.3 What a custom Grand Prix cup needs

Choose among the 8 existing cup slots (no new slot without code changes, section 7):

1. Write the 4 course ids (u32) of the slot at 0x02154128 + cup*16: ARM9 data patch, no code change [C: table layout].
2. Course name text: the name shown for a race is taken by flat position, so edit common.bmg messages 45+pos to the new names (only for the replaced positions) [C mapping, I for exact use].
3. Cup name: BMG ids 37..44, plus the localized sprite art (`select_cup_name_m`, `select_cup_m`, `select_cup_s`) [C location, I how they combine].
4. Cup icon/trophy: `data/Scene/Menu.carc/common/cup_mark.nce.ncgr` (4,144 B, shared across languages) and `trophy_icon.nce.ncgr` (2,096 B) hold the cup/trophy marks; `select_cup_m_b.NCGR`/`select_cup_m_picture.NSCR` is the cup-select backdrop. Which tile is which cup is not decoded [I].
5. Course preview pictures: `data/CupPicture/select_cup_{nitro,retro}NN_m_*` where NN = flat position within its family + 1 (nitro cups 0..3 -> 01..16, retro cups 4..7 -> 01..16); the new track needs a new NCGR/NCLR/NSCR set at the position it occupies [C naming/count, I order].
6. Music: via the course's music record (section 3), staff ghosts: `ghost_NNN.bin` and staff_ghost_time are per flat position (ghost slot = flat position [I]); delete/ignore them for a replaced track.
7. Save data: best-time records are per course/cup position (RaceHistory has a u16[8] course table field [C name], so retail progress for the old track is reused under the new one unless the save is reset [I]).

## 7. Adding extra track slots (instead of replacing)

Limits and tables a code patch (as CTGP-Nitro does) must change [C unless marked]:

| Limit | Where | What to change |
|---|---|---|
| Course ids 0..54 (55 table entries; id range checked as <= 0x35 in the music switch) | course table 0x02154228 (next table starts 8 bytes after the last entry, so it cannot grow in place); 2 literal refs | relocate the table to free memory (end of ARM9 or an overlay), update the 2 literal pool words, and fix any `cmp #0x36`-style bounds [I] |
| Music switch | code at 0x0202553C, 54-entry branch table at 0x0202554C, default index 0x36; record table at 0x0215316C has 51 records | replace the switch with a table lookup (fits: ARM code size 0x23C bytes) and extend the 4-byte record table (relocate; 3 literal refs: 0x02025530, 0x02025778, 0x02025788) |
| Cups: 8 x 4 = 32 flat positions | cup table 0x02154128 (9 literal refs), loops with bounds 8 and 4 in `course_find_cup` / `course_find_cup_position` and the progress code | enlarge the table and the loop bounds; flat position range 0..31 is also an index space for ghost slots (32), staff times (32 x 5 B), CupPicture sets (32), BMG course-name messages (32 at 45..76), TT/records screens |
| Ghost files | `data/Ghost/ghost_000..031.bin` (32) | one per cup course; new slots need new files and filename logic [I] |
| Save data | per-course best times/ranks indexed by course or flat position; RaceHistory u16[8] table | the save layout would have to grow or reuse slots; extending the SRAM/flash save is risky (CRC, fixed block sizes) [I, not inspected] |
| Menu UI | cup-select screen layout shows 4 cups per page x 2 (Nitro/Retro tabs) and 4 course slots | overlay code, sprite art per language, NSCR/cell banks: a large patch; the practical route is replacing within the 32 existing positions or showing extra courses in a separate "custom" menu [I] |
| Archive naming | `/data/Course/%s%s%s.carc` | the new course only needs its archive pair and a name pointer; new D variants only if kind == 1 |
| Music | SEQ ids are u8 in the record (<=255, 76 used) | no limit problem; add SSEQ/bank/wave files as in section 3.4 |
| Objects/lap | STAG +0 holds the course id (u16) | set it to the new id |

Minimum viable "extra slots" without UI changes: keep 32 cup positions but allow cup rows to be rewritten (that is the replace-in-place workflow) and add free-choice courses only for VS/Time Trial via a custom list [I].

## 8. Open questions

1. Meaning of the aux byte (byte 2) of the music record, and exact final-lap mechanism (tempo variable vs second sequence).
2. Table that maps NKM object id to class/model files (only handler tables 0x0216B94C / 0x0216B9AC found); names/semantics of ids other than 0x65.
3. Resolved for the generated local layer (2026-11-04): `minimap_course_corners` at 0x020B6A18 reads the 55 x 4 signed-short rectangle table at RAM 0x0216890C, indexed by course*8. The second rectangle table starts at 0x02168754. The wizard fits the source XZ bounds inside the first rectangle and rasterizes `(x-left)/(right-left)` and `(z-top)/(bottom-top)` into its 256x256 local layer. Existing global/second layers are retained; verify those overlays on the selected course in game. Heights and overpasses require manual route inspection.
4. Exact meaning of KCL attribute bits 0-4 and 13 (0x2000); which type is lava.
5. Which tiles of `cup_mark`/`trophy_icon`/`select_cup_*` correspond to each cup; where the in-race course-name banner text comes from.
6. Sound heap behaviour above ~300 KB per song and whether sound mode 2 is the race mode: needs a boot test.
7. Overlay references to the course/cup tables (overlays are compressed, not scanned).
8. Save-data layout per course (fields not inspected here).

## 9. Generated track heap limits (2026-11-04)

The maintainer observed a 499 KB generated KCL crash on race load; coarsening it to
55 KB booted. Generated KCL now uses a grid of 1024-unit cubic roots (coordinate
shift 10, nonzero Y/Z index shifts), conservative expanded-AABB candidates, shared
leaf lists and bounded coarsening. It warns above 150,000 bytes and refuses above
215,000 bytes. These conservative generated-file caps apply even though the maximum
across all 59 retail course archives (including battle/unused courses) is 287,602 bytes.

Every generated member also stays at or below the largest expanded retail member
with that exact relative path, measured from the user's ROM in memory. Observed
maxima: course model 124,952 B, textures 129,780 B, NKM 21,960 B, local NCGR 65,600 B,
local NCLR 570 B, local2/local3 NSCR 8,228 B each. The KCL cap is additionally enforced
by export validation. Sizes are metadata only; no retail content is stored here.

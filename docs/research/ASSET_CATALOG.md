# MKDS (USA, AMCE rev 0): asset catalog for plain-English labels

Structure only. No ROM bytes, images or message text are reproduced.
Tags: **[C]** confirmed (file names, archive structure, sizes, or the earlier research docs `CHARACTERS_KARTS.md` / `TRACKS_AUDIO_CUPS.md`), **[I]** inferred (naming pattern or well-known game knowledge, not proven in this ROM).

Source: `mkdstb ls clean.nds` (4,880 rows) = **286 `.carc` archives + 320 loose files + 4,274 archive members**. 140 of the 320 loose files are a loose copy of `KartModelMenu.carc` (note 1 in section 1). The lister labels a file `bin` when it does not recognise the format; extensions below come from the file names.

## 1. Folder guide

| Path | Files | What it holds | Where the player sees it |
|---|---|---|---|
| `data/Course/<name>.carc` | 59 | Track geometry, collision, logic (`course_model.nsbmd`, `course_collision.kcl`, `course_map.nkm`), sky model, track objects (`MapObj/`), mission-mode logic [C] | The race track itself |
| `data/Course/<name>Tex.carc` | 59 | Track textures (`course_model.nsbtx`), sky textures, minimap (`Map2D/`) [C] | Track colours; minimap on the touch screen |
| `data/CupPicture/` | 96 loose | 32 course preview pictures (16 `nitro`, 16 `retro`), 3 files each (tiles, palette, screen) [C] | Course pick screen inside a cup |
| `data/KartModelMainA.carc` | 104 | Race karts and drivers for MR YS WR PC KP KO DK, shared tires, shadows, emblems, Bullet Bill, physics/AI tables [C] | Racing |
| `data/KartModelMainB.carc` | 39 | Same for WL RB LG DS KA, plus the full Shy Guy (HH) driver set [C] | Racing |
| `data/KartModelSub.carc` | 72 | 12 detailed driver models + 48 animation clips [C] | Driver in the kart during races |
| `data/KartModelMenu.carc` and loose `data/KartModelMenu/` | 140 each | Menu-quality karts, drivers, win clips, floor ring, menu stat tables [C] | Character/kart select screens |
| `data/KartModelMain.carc` | 136 | Developer superset (adds `OY` and extra `HH` karts); code never references this name [C per earlier research] | Never |
| `data/MainRace.carc` | 34 | Item 3D models, item box parts, bomb effect models, rank tables, `ItemSlotList.dat` [C] | Held/thrown items, explosions |
| `data/Main/MapObj.carc` | 9 | Item box and shadow models + animations [C] | The item boxes on the track |
| `data/MainEffect.carc` | 1 | `RaceEffect.spa` particle archive [I: `.spa` = particle data] | Dust, sparks, smoke |
| `data/Main2D.carc` (neutral) | 7 | Fonts, kart unlock table, Shy Guy palettes, mission table [C] | Used by all menus and HUD |
| `data/Main2D_<lang>.carc` | 5 | `common.bmg` text plus small character-icon sprites (`player_character_L/M`) [C] | Name/rank lists, results |
| `data/Static2D.carc` | 21 | Download-Play child screens: icons, per-language text and fonts (includes `_ja`) [C] | Download Play on a child DS |
| `data/CharacterKartSelect*.carc` | 6 | Character and kart pick screen art; `_<lang>` holds the text-bearing sprites and `kart_select.bmg` [C] | Pick character / kart |
| `data/GeneralMenu*.carc` | 6 | Shared menu backgrounds, text-box frames, back and choice buttons (11 screens, 6 tile sets) [C]; the 5 language copies are empty placeholders [C] | Behind almost every menu |
| `data/Scene/*.carc` | 141 | One archive per UI "scene" (section 4.4); 23 families x (1 neutral + 5 language) + `Edit`, `Name`, `Test` [C] | Title, menus, HUD, results, credits |
| `data/MissionRun/` | 43 loose | Mission-mode boss and object models, animations, `.prm` parameters, `MissionEffect.spa` [C] | Mission mode bosses and gates |
| `data/Ghost/` | 33 loose | `ghost_000..031.bin` (4,096 B each) + `staff_ghost_time.bin` (160 B) [C] | Staff ghosts in Time Trial [I] |
| `data/Sound/sound_data.sdat` | 1 (3,321,568 B) | All music, sound effects, voices [C] | Everywhere |
| `data/Boot/` | 5 | `dbgfont` (debug font), `builddate.bin`, `mkds_banner02.nbfc/.nbfp` (512 B image + 32 B palette) [C names, I meaning] | Debug builds / DS menu icon |
| `data/dwc/utility.bin`, `dwc/utility.bin` | 2 (1,069,000 B each) | Nintendo Wi-Fi library blob, two copies of the same size [I] | Wi-Fi connection |

Notes:
1. **KartModelMenu exists twice.** Loose `data/KartModelMenu/*` and the carc hold the same 140 paths, but 42 differ in size (36 `nsbmd`, 5 `nsbtx`, 1 `bin`) [C]. Earlier research says the game mounts the loose folder [C]; so the carc is probably stale [I]. The Toolbox should edit the loose copy and warn about the carc.
2. 7 families have only empty 85-byte language archives (35 files): `GeneralMenu`, `Ending`, `Logo`, `Nickname`, `Race_gp`, `Race_nt`, `Race_vs`. Each holds one 0-byte `dummy.txt` [C]. `Scene/Name.carc` is also an empty 85-byte archive [C].
3. `data/Course` also holds 3 non-track archives that are laid out like tracks: `Award`, `StaffRoll`, `StaffRollTrue` [C].

### Course archives (59 pairs = 118 files) [C counts]

| Group | Count | Plain label | Where seen |
|---|---|---|---|
| `*_course` with a cup slot (Nitro: cross, bank, beach, mansion, desert, town, pinball, ridge, snow, clock, mario, airship, stadium, garden, koopa, rainbow) | 16 | New (Nitro) Grand Prix track | Cups 1-4 |
| `old_*` with a cup slot | 16 | Classic (Retro) track | Cups 5-8 |
| `mini_*` (`block_course`, `block_64`, `dokan_gc`, `stage1..4`) | 7 | Battle arena | Battle mode |
| `MR_stage1..4` | 4 | Mission Run stage | Mission mode |
| `Award`, `StaffRoll`, `StaffRollTrue` | 3 | Podium scene, credits roll, true-ending credits roll [I] | Cup awards, credits |
| `old_mario_gc`, `luigi_course`, `dokan_course`, `test1_course`, `donkey_course`, `wario_course`, `nokonoko_course`, `test_circle` | 8 | Unused development tracks (ids 2..8 and 21; no music record) [C ids; I "unused"] | Not reachable in normal play |
| `*D` (`beach_courseD`, `cross_courseD`, `mansion_courseD`, `old_luigi_gcD`, `old_momo_64D`) | 5 | Alternate version of a track; purpose unknown [I] | Unknown |

## 2. Naming conventions

### 2.1 Prefixes and suffixes

| Pattern | Meaning | Status |
|---|---|---|
| `P_XX` | Player: detailed, animated driver model (`P_XX.nsbmd/.nsbtx`) | [C] |
| `E_XX` | Low-detail static driver (1 node; what "E" stands for is unknown) | [C] |
| `P_XX_drive` / `_spin` / `_win` / `_lose` | The 4 animation clips (steering, hit spin, goal win, goal lose) | [C] |
| `P_faceanim.nsbtp` | Shared eyes open/closed switch for all `P_` drivers | [C] |
| `kart_XX_a/b/c` | Kart body. `a` = shared Standard kart (MR only), `b` / `c` = the character's two karts | [C] |
| `kart_tire_S/M/L` | Tire size (small, medium, large) shared by all karts | [C] |
| `sh_XX_a/b/c` | Kart shadow mesh, one per kart (no texture) | [C] |
| `XX_emblem.nsbtx`, `TeamRed/Blue_emblem` | Emblem decal texture per character, and the two team colours | [C] |
| `KillerItem*`, `it_killer` | Bullet Bill | [C] |
| `it_*` | Item model (`banana`, `bomb`, `gesso`=Blooper, `kinoko`=mushroom, `kinoko_p`, `koura_g/r/w`=shells, `star`, `teresa`=Boo, `thunder`, `f_box`, `killer`) | [C names; I meanings from Japanese names] |
| `T_<cup>_a/b/c`, `T_<cup>` | Trophy models in Result (cups: `bana flow kino kono kora spec star thun`); a/b/c probably gold/silver/bronze | [C names; I meaning] |
| `<name>_course` | Nitro track. Same stem + `Tex` = its textures | [C] |
| `old_<name>_<sfc/64/gc/agb>` | Classic track; suffix = source console: `sfc` Super NES, `64` Nintendo 64, `gc` GameCube, `agb` Game Boy Advance | [I] (well-known; not stated in ROM) |
| `mini_*` | Battle arena | [C] |
| `MR_stage*`, `MissionRun/mrNN_tool.nkm` | Mission Run stage; per-mission map variants (65 files in 43 courses) | [C] |
| `D` after the track name | Variant archive pair; only the 5 courses with course-table kind 1 | [C exist; I purpose] |
| `Tex` | Texture partner of the same-named archive | [C] |
| `course_model` / `course_model_V` | Track mesh / sky ("V" = vista) | [C names; I "sky"] |
| `Map2D/global*`, `Map2D/local*` | Minimap layers: `global` = shared frame/background, `local` = this course's image | [I] |
| `_m` / `_s` in 2D art | Main (top) / Sub (bottom touch) screen | [I] |
| `_b` / `_bg` / `_o` in 2D art | `_b` = background tiles/palette, `_bg` = background screen map, `_o` = sprite (OBJ) tiles/palette | [I] |
| `.nce.ncer/.nce.ncgr` | Sprite cell bank + tiles exported as a pair | [C] |
| `.bncl .bnll .bnbl` | Layout side-files for a sprite pair (cell list / layout / bank list) | [I] |
| `.NFTR`, `.bmg`, `.kbdmap`, `.prm`, `.spa`, `.tbl` | Font, text, keyboard layout, boss parameters, particles, table | [C ext; I meaning] |
| `player_character_*_o.NCLR` | Palette for that sprite sheet; `_shyguy_o` is the Shy Guy alternate | [C] |
| Capital letters differ (`Select_game_m`, `Results_battle_*`) | Same naming, different case; match stems case-insensitively | [C] |

### 2.2 Language suffixes

| Suffix | Language | Where | Status |
|---|---|---|---|
| (none) | Language-neutral: palettes, backgrounds, 3D, shared art | every scene | [C] |
| `_us` | **English (US), the one to show** | 26 families, 130 archives total | [C] |
| `_es` | Spanish | same | [C] |
| `_fr` | French | same | [C] |
| `_ge` | German (not `_de`) | same | [C] |
| `_it` | Italian | same | [C] |
| `_ja` | Japanese | only `Static2D.carc/MBChild_ja.*` | [C] |

26 families have a neutral archive plus 5 language archives. 7 of them (35 archives) are empty placeholders. Text-bearing sprites (`_o.NCGR`, `.nce.ncgr`, titles, labels, cup names) and every `.bmg` live in the language archive; palettes and shared art stay in the neutral one [C]. There are **no `_jp` or `_uk` archives** in this ROM. Note: `_us`/`_es`/... are English-first only for `_us`; the other four are other languages, not alternative English.

### 2.3 Character codes [C] (from `CHARACTERS_KARTS.md`)

| MR | DK | KO | KP | PC | WR | YS | LG | KA | DS | WL | RB | HH | OY |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Mario | Donkey Kong | Toad (kinopio) | Bowser (koopa) | Peach | Wario | Yoshi | Luigi | Dry Bones (karon) | Daisy | Waluigi | R.O.B. (robo) | Shy Guy (heyho) | developer leftover (oyama) |

Folder names are the lower-case words: `mario donkey kinopio koopa peach wario yoshi luigi karon daisy waluigi robo heyho` (`oyama` dev only).

## 3. Pairing rules the tool can apply

Reliability: **High** = holds on every retail example; **Med** = holds except named exceptions; **Low** = heuristic.

### 3.1 2D art (NCLR / NCGR / NSCR / NCER / NANR)

| Rule | Reliability | Examples | Exceptions |
|---|---|---|---|
| Same stem, same archive: `X_b.NCGR` + `X_b.NCLR` (+ `X_bg*.NSCR`) = a background image. | High | `emblem_s_b`, `select_menu_etc_m_b` (+ `_bg_00/_01`) | NSCR sometimes has no `_bg` (e.g. `select_kart_s.NSCR` + `select_kart_s_b.NCGR`) |
| `X_o.NCLR` + `X_o.NCGR` + `X.NCER` (or `X.nce.ncer/ncgr`): sprite sheet; the cell file has no `_o`. | High | `race_m_o` + `race_m.NCER`; `select_character_s_o.NCLR` + `select_character_s.nce.*` | A few palettes (`Option_embrem_select_s_o.NCLR`) serve a `.nce` pair with a different stem |
| **Palette in the neutral archive, tiles in the `_us` archive.** Join `Scene/Foo.carc` and `Scene/Foo_us.carc` by stem. | High (English) | `Race_ta.carc/race_new_record_ta_s_o.NCLR` + `Race_ta_us.carc/race_new_record_ta_s_o.NCGR`; `Title.carc/title_s_b.NCLR` + `Title_us.carc` screens | `Title.carc` has Europe-only maps (`title_m1_EU.NSCR`), `Title_us` has its own `title_m1.NSCR`: a screen map may be in either |
| NSCR beside a tile file of the same family; match by stem prefix before `_bg`. | Med | `GeneralMenu`: 11 NSCR share 6 tile sets via `_00/_01/_02` numbering | 240 NSCR have no same-stem NCGR in their archive, mostly minimap `Map2D/local2/3`, `global1/2` |
| Minimap: `Map2D/local.NCGR` + `local.NCLR` + `local2.NSCR` (+ `local3`); `global.*` + `global1/2.NSCR`; `global2.NCGR/NCLR` | High | all 59 `*Tex.carc` | `old_kinoko_gcTex` adds `old_luigi_gc_top.NCLR`; 4 courses lack `global` |
| NANR animation pairs with the NCER of the same stem | High but rare | `emblem_s`, `emblem_m`, `emblem_camera_s` (Emblem, Edit), `Keyboard_s_o` (Nickname) | Only 5 NANR exist; every other 2D animation lives in the `.bn*` side files (not decoded) |
| `CupPicture`: `select_cup_<nitro/retro>NN_m_b.NCGR/.NCLR` + `..._m_picture.NSCR` | High | 16+16 sets | none |
| Many `_o.NCLR` have no tile partner in the same archive | Low | `Menu/common/*_s_o.NCLR` | tiles are in `Menu_us` or another `.nce` pair; do not warn about a missing partner |

### 3.2 3D models and animation

| Rule | Reliability | Examples | Exceptions |
|---|---|---|---|
| Kart, tire, E_ and P_ models: `X.nsbmd` + `X.nsbtx` (same stem, same archive). | High | `kart_MR_b`, `kart_tire_M`, `E_PC`, `P_YS` (78/78 karts incl. dev copies, 9/9 tires, 41/42 drivers) | `KillerItem_s.nsbmd` shares `KillerItem.nsbtx`; shadows (`sh_*`) have no texture by design |
| Driver clips: model stem + `_drive/_spin/_win/_lose` in the same folder. | High | all 12 in `KartModelSub`, `P_HH_*` in MainB | KartModelMenu holds only `_win`; E_ models have none |
| `P_faceanim.nsbtp` applies to every `P_` driver. | High | single file in MainA | none |
| Kart emblem: character code `XX` of the kart picks `emblem/XX_emblem.nsbtx` (applied to the kart's `kart_emblem` material at run time). | Med | `kart_DK_b` -> `DK_emblem` | Kart `a` (MR) and team emblems follow game rules, not file names |
| Kart tire and shadow: tire size from `kartoffsetdata.bin` (id%3: 0=M, 1=S, 2=L); shadow stem `sh_<same code>_<a/b/c>`. | High (shadow), Med (tire) | `sh_MR_a` for `kart_MR_a` | `sh_OY_*` only in dev superset; `sh_KA_*` only in A and Menu |
| Course model: `<name>.carc/course_model.nsbmd` + `<name>Tex.carc/course_model.nsbtx`. | High | 50/59 | **9 have no nsbtx** (textures embedded in the nsbmd): dokan, donkey, luigi, mini_block_course, nokonoko, old_mario_gc, test1, test_circle, wario |
| Course sky: `course_model_V.nsbmd` + `Tex.carc/course_model_V.nsbtx`; texture animation `course_model(_V).nsbta` in the base archive. | High | 50 sky models, 47 with nsbtx | 9 courses have no sky model (StaffRoll, StaffRollTrue, dokan, mansion_courseD, mini_block_course, nokonoko, old_mario_gc, test1, test_circle); donkey, luigi, wario have a sky with embedded textures |
| Course collision and logic: `course_collision.kcl` and `course_map.nkm`, one each in the base archive. | High | 59/59 | `MissionRun/mrNN_tool.nkm` are extra NKM files (65 in 43 courses) |
| Course objects: `MapObj/X.nsbmd` with same-stem `.nsbtp/.nsbca/.nsbma/.nsbta`. Textures embedded (251/251 without nsbtx). | High | `water_efct` trio in 18 courses, `kuribo` + `.nsbtp` | `old_hyudoro_64/MapObj/bk_wave` has `.nsbca+.nsbma` and no model |
| Item models (`it_*.nsbmd`): textures embedded; only `it_bomb.nsbtp` animates. | High | all 15 | none |
| Race banners: model in the `_us` archive, animation in the neutral one (`goal`, `start`, `win`, `lose`, `complete`, `failed`). | High | `Race.carc/goal.nsbca` + `Race_us.carc/goal.nsbmd`; `Race_mr` -> `Race_mr_us` | `count.nsbmd` is in the neutral archive; `countD.nsbtp` has no model |
| Result trophies: `T_<cup>.nsbmd` + `T_<cup>.nsbtp`; `T_<cup>_a/b/c.nsbmd` have no animation. | Med | 8 cups x 4 | `ResultDL.carc/trophy.nsbmd` is a single trophy |

### 3.3 Cross-archive links

| Link | Rule | Reliability |
|---|---|---|
| Course -> cup picture | course-table id -> flat cup position -> `select_cup_<nitro/retro>NN` (NN = pos+1 within Nitro/Retro) | High [C, earlier research] |
| Course -> music | course-table id -> music slot table (`TRACKS_AUDIO_CUPS.md` section 3.3) | High |
| Course -> name text | `common.bmg` ids 45..76 = flat cup position + 45 (US) | High |
| Character -> sound, HUD icon | code order = character id; sprite cell index = id | Med [I] |
| Kart -> physics | `kartphysicalparam.bin`, `kartoffsetdata.bin` indexed by kart id = 3*group+k | High |
| Character -> kart unlock | `Main2D.carc/kart_appear.ktbl` (37 rows x 13 columns) | Med |

## 4. Plain labels by category ("where you see it")

### 4.1 Characters

| File type | Label |
|---|---|
| `P_XX.nsbmd/.nsbtx` (KartModelSub, plus HH in MainB) | "Driver (detailed): the character sitting in the kart during a race" (13 characters) |
| `P_XX_drive` | "Driver steering pose" |
| `P_XX_spin` | "Driver hit-spin reaction" |
| `P_XX_win` | "Driver victory animation after the race" (also on the select screen) |
| `P_XX_lose` | "Driver defeat animation after the race" |
| `E_XX.nsbmd/.nsbtx` | "Driver (simple): low-detail driver used when detail is off" [C: used when the detailed flag is 0] |
| `P_faceanim.nsbtp` | "Eyes open / eyes closed (shared)" |
| `KartModelMenu/character/*/P_XX.*` | "Driver on the character select screen" |
| `HH_color.NCLR` | "Shy Guy colour palette" |
| `Main2D_us.carc/player_character_L/M` | "Character head icons (large / medium) in lists and results" [I] |
| `Main2D.carc/player_character_*_shyguy_o.NCLR` | "Shy Guy icon colours" |

### 4.2 Karts

| File | Label |
|---|---|
| `kart_MR_a` | "Standard kart (shared body, recoloured per character)"; palette `kart/palette/kart_a.NCLR` [C file; I recolour] |
| `kart_XX_b`, `kart_XX_c` | "<Character> kart 2 / kart 3 body" |
| `kart_tire_S/M/L` | "Kart wheels: small / medium / large" |
| `sh_XX_*` | "Kart ground shadow (no picture)" |
| `XX_emblem.nsbtx`, `TeamRed/Blue_emblem` | "Emblem sticker on the kart / team emblem" |
| `emblem_s_b.NCLR` | "Emblem palette" |
| `KillerItem*` | "Bullet Bill (kart-shaped item)" |
| `kart/select/select.*` (Menu) | "Floor ring under the kart on the select screen" |
| `kartoffsetdata.bin` | Technical: wheel and seat positions |
| `kartphysicalparam.bin`, `kartAIparam.bin`, `kartcrsmat.bin`, `charphysicalparam.bin` | Technical: speed/handling, CPU driver, surface, character stats |
| `kartmenuparam.bin`, `charmenuparam.bin` | Technical: select-screen stat bars [I] |
| `kart_appear.ktbl` | Technical: which kart each character can use / unlock tier |
| `dbgprim.nsbmd` | Debug shape (never shown) [I] |

### 4.3 Courses

| File | Label |
|---|---|
| `course_model.nsbmd` | "Track 3D model" |
| `course_model.nsbtx` (Tex) | "Track textures" |
| `course_model_V.nsbmd/.nsbtx` | "Sky and distant scenery" |
| `course_model.nsbta`, `_V.nsbta`, `.nsbtp` | "Scrolling water/lava/lights (texture animation)" |
| `course_collision.kcl` | "Where the track is solid and what each surface is (collision)" |
| `course_map.nkm` | "Track layout: start line, checkpoints, item boxes, enemy paths, cameras" |
| `MissionRun/mrNN_tool.nkm` | "Mission Run version of this track layout" |
| `MapObj/*` | "Objects on this track (trees, Goombas, waves, ...)" |
| `Map2D/local.*`, `local2/3.NSCR` | "Minimap image (this track)" |
| `Map2D/global*` | "Minimap frame shared across tracks" [I] |
| `CupPicture/select_cup_*` | "Picture of this track on the cup screen" |
| `Ghost/ghost_NNN.bin` | "Staff ghost record" [I] |

### 4.4 Menu and HUD screens (`data/Scene/<Name>[_lang].carc`)

| Group | Members neutral / `_us` | Plain label | Where seen |
|---|---|---|---|
| `Title` | 8 / 12 | Title screen logo and prompts | Boot / title |
| `Logo` | 9 / 0 | Nintendo and ESRB logos | First screens |
| `Menu` | 84 / 92 | Main menu: mode, rules, cups, Time Attack, mission | Single/multiplayer menus |
| `MenuDL` | 23 / 21 | Menu variant for wireless Download Play | Download Play |
| `Option` | 11 / 24 | Options screens | Options |
| `Record` | 44 / 52 | Records and best times, trophies, erase | Records |
| `Result` | 52 / 10 | Grand Prix results, trophies, cup names | After a cup |
| `ResultDL` | 27 / 22 | Battle/VS results, trophy | After battle / multiplayer |
| `Race` | 56 / 13 | In-race HUD: countdown, GOAL, lap, item roulette, pause | Racing |
| `Race_gp`, `Race_mg`, `Race_mr`, `Race_nt`, `Race_ta`, `Race_vs` | 3/0, 5/1, 14/3, 1/0, 7/1, 0/0 | HUD overlays: Grand Prix start, Battle, Mission Run, (nt unknown), Time Attack, VS (empty) | Racing, per mode |
| `WiFiMenu`, `WLMenu` | 31 / 22, 13 / 15 | Wi-Fi and wireless multiplayer menus | Online / wireless |
| `Ghost` | 26 / 18 | Ghost send/receive | Time Trial ghosts |
| `Emblem`, `Edit` | 31 / 5, 13 / 0 | Emblem editor and emblem selection | Emblem creation |
| `Keyboard`, `Nickname` | 39 / 6, 37 / 0 | On-screen keyboard and name entry | Naming |
| `Ending`, `StaffRoll` | 28 / 0, 6 / 2 | Thank-you screens, credits | Credits |
| `Test`, `Name` | 7 / 0, 0 / 0 | Developer test fonts; empty | Never |
| `CharacterKartSelect*` | 17 / 20 | Character and kart pick screens | Pick screens |
| `GeneralMenu` | 22 / 0 | Shared menu background and text frames | Everywhere |

Member counts are by archive, with `dummy.txt` placeholders excluded from the `_us` column. [C counts, I labels from names]

### 4.5 Item graphics

| File | Label | Where |
|---|---|---|
| `MainRace/Item/it_*.nsbmd`, `koura_w`, `geso_sumi` (15 models) | "3D item: held, thrown or dropped" | On track |
| `Main/MapObj.carc/itembox*` | "Item box and its shatter pieces" | On track |
| `MainRace/MapObj/box`, `question` | "Item box / question mark" | On track |
| `Race_us.carc/race_s_o.NCGR` + `Race.carc/race_s_o.NCLR`, `race_s.NCER` | "Item icons, digits (HUD sheet, touch screen)" [C, earlier doc] | Item slot |
| `Race.carc/race_m_o.*` | "HUD sheet, top screen" [I] | Top screen |
| `MainRace/Item/ItemSlotList.dat` | Technical: item odds by rank | - |
| `MainRace/effect/bombcore*`, `hanegame`, `award_track` | "Explosion effect / small effect textures" [I] | Bomb hit |

### 4.6 Fonts

| File | Label | Where |
|---|---|---|
| `Main2D.carc/LC_Font_m.NFTR`, `LC_Font_s.NFTR` | "Normal text font (medium/small)" [I: LC = Latin chars] | Menus, HUD |
| `Main2D.carc/marioFont.NFTR` | "Mario-style big title font" [I] | Titles, numbers |
| `Scene/Test.carc/LC_Font_l/m`, `marioFont`, `selectFont` | "Developer test copies of fonts" | Never |
| `Scene/Edit.carc/selectFont.NFTR` | "Emblem editor font" [I] | Emblem editor |
| `Static2D.carc/MBChild_<lang>.NFTR` | "Download Play screen font per language" | Download Play |
| `Boot/dbgfont.NCGR/.NCLR` | "Debug font (picture)" | Never |

Fonts are `.NFTR` files the lister reports as `bin`; the tool needs an NFTR reader to preview them (14 files).

## 5. Counts and the Advanced bucket

### 5.1 Per category (unique files; dev superset and the loose KartModelMenu duplicate excluded unless noted)

| Category | Count |
|---|---|
| Course archive pairs | 59 (32 cup, 7 battle, 4 mission, 3 award/credits, 8 unused, 5 D variants) |
| Course members | nsbmd 109 (59 track + 50 sky), kcl 59, nkm 59 (+65 mission nkm), course nsbtx 97, MapObj nsbmd 251 |
| Minimaps | 59 `local`, 55 `global` |
| Detailed drivers (`P_`) | 13 models + 52 clips + 1 face clip |
| Simple drivers (`E_`) | 13 |
| Karts / tires / shadows / emblems | 25 / 3 / 25 / 15 |
| Menu copies (loose and carc, each) | 12 drivers + 12 win clips + 25 karts + 3 tires + 25 shadows + 15 emblems |
| Item models | 15 + 3 item box models + 2 box/question |
| Trophy models | 33 |
| Scene archives | 141 (23 families x 6 + Edit, Name, Test) |
| 2D art, all languages | NCGR 672, NCLR 432, NSCR 470, NCER 284, NANR 5 (distinct after merging languages: 389 / 380 / 374 / 100 / 5) |
| Text (`.bmg`) | 92 files (23 distinct paths across languages) |
| Cup pictures | 96 files (32 pictures) |
| Fonts | 14 `.NFTR` + 1 debug tile font |
| Sound | 1 SDAT |

### 5.2 Advanced bucket (no friendly picture or label)

| Group | Files | Why |
|---|---|---|
| `.bncl` / `.bnll` / `.bnbl` layout side files | 846 (194 distinct names) | Format not decoded; companions of `.nce` sprite pairs [I] |
| Keyboard `.kbd` 10, `.kbdmap` 28, `.lld` 4 | 42 | Key layouts; formats not decoded |
| Param/table: other `.bin` 52, `.prm` 8, `.tbl` 3, `.mtbl`, `.ktbl`, `.rpt`, `.rtt`, `.dat`, `.mr` (1 each) | 70 | Technical tables, boss parameters |
| Particle `.spa` | 3 (`RaceEffect`, `RaceEffectFogOff`, `MissionEffect`) | Particles |
| `dummy.txt` | 66 (0 B placeholders) | Hide |
| Ghost bins | 33 | Binary ghost data |
| `Boot` banner, `builddate.bin`, `dwc/utility.bin` x2 | 5 | Not user content |
| NCGR with no NCLR in the same group | 7: `dbgfont`, `emblem_template`, `Keyboard_s_b_flame` (x2), `Menu/common/Select_game_m`, `race_transfer_s` | Palette is in a sibling archive |
| NCLR with no NCGR in the same group | 28 groups, e.g. `kart_a`, `HH_color`, `emblem_s_b`, `old_luigi_gc_top` | Palette-only (shared) |
| Orphan animations | `bk_wave`, `countD.nsbtp`, `P_faceanim`, and the banner animations whose model is in `_us` | Shared or cross-archive |
| Developer-only content | `KartModelMain.carc` (136), 8 unused tracks, `Scene/Test`, `Scene/Name` | Hide by default |
| Duplicate: KartModelMenu loose vs carc | 140 pairs, 42 differ | Warn |

## 6. Summary

- **Layout:** 286 archives hold 4,274 members; 320 loose files sit in `data/`. Organisation is archive-per-screen (`Scene`), archive-pair-per-track (`Course`), and archive-per-asset-class (`KartModel*`, `Main*`).
- **Strong auto-pairing rules:** model + texture by stem; track + `Tex` partner; `P_` model + `_clip` suffix; NCLR/NCGR/NSCR by stem and `_b/_bg/_o`; English is `_us`.
- **Biggest trap:** palettes in the neutral archive and tiles/text in the `_us` archive. Always merge `X.carc` + `X_us.carc` before looking for a partner.
- **Other traps:** 9 tracks have embedded textures, 9 have no sky, KartModelMenu exists twice with 42 differing files, 7 language families are empty, and only 5 NANR files exist.
- **Uncertain [I]:** purpose of the five `D` track variants; meaning of `E_` and `LC_`; which KartModelMenu copy is used (loose, per earlier research); `Race_nt` content; trophy a/b/c order; the `bn*` layout files; the `global` minimap role; `.spa` format; staff-ghost mapping. Counts for 2D partner statistics come from a stem heuristic, so treat them as approximate. Plain course names for `old_*` and `mini_stage*` are from game knowledge, not the ROM, and are left out of the tables.

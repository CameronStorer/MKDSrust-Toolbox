# Toolbox status - 2026-11-05, Stage 6B Run 1

Real hardware first. DS ROM/BPS export still uses the existing validated ROM rebuilder.
The user's ROM is read in place through MKDS_ROM; no ROM-derived bytes or screenshots
were written to this repository. No hardware/emulator boot verification is claimed.
F, G and H's new editing features are left for later.
The existing camera preservation, Simple | Studio and FileLink/openAsset are retained.

| Section | Done in Run 1 | Partial / remaining |
|---|---|---|
| A - Explorer | Expand/collapse all; folder-preserving type sorting, icons and friendly types; descendant counts; English-first language toggle; expansion, query, selection and scroll survive workspace changes; double-click editor routing and unknown-format Export popup. Shared file links now cover history, changes, validation, wizard/export reports and warnings, revealing the file in the tree. | Folder hints are structural counts, not a hand-written description of every folder. Unknown formats retain hex preview/export. |
| B - pairing | Naming rules live in mkds-data; indexed, cached associations merge neutral/localized archives, sprite stems and minimap/cup-picture patterns, with image depth/tile-index compatibility checks. Pairing is shown in Details; image controls allow overrides. Models use their own textures and clips, course Tex partners, automatic driver playback and the existing animation picker. Kart views attach the selected tire model at the four table offsets. | Shared face NSBTP is linked but eye switching/NSBTA/NSBMA playback is not implemented. Some exceptional cross-family image pairs remain unresolved. Manual image overrides affect the current view; they are not saved as project metadata. Emblem/shadow relationships are shown on related pages; runtime emblem composition remains partial. |
| C - related pages | Related assets is available in Simple and Studio and linked from Game browser. Character pages show race/far/menu variants, icon cells, voice-bank sample playback and kart links. Kart pages show race/menu bodies, S/M/L tires, shadow, emblem and stats. Catalogued course pages show model, sky, collision, minimap, routes/objects and music. Each card has its own viewport; offscreen cards release graphics resources. Replace opens the existing recipe for that slot; image replacement opens the image tools. | Icon-cell order remains labelled inferred; missing roles are explicit. Voice preview plays samples from the voice-event bank, not a complete named event sequence. Course pages currently cover loaded course-table entries with models, not every D variant/award/credits scene. Wheel placement uses the existing 16:1 picker conversion and still needs visual confirmation against gameplay. Shy Guy's special kart replacement is disabled. No new dedicated tire/emblem/voice wizard. |
| E - small packages | Format 3 writes one BSDIFF40 per changed decompressed archive member, including nested CARCs, with source/result SHA-256 checks. A bounded in-memory reference consumer repacks/recompresses; disjoint member edits combine and wrong sources fail atomically. Nonarchive packages remain v1. Stale menu-archive edits warn; browser pages, openAsset and recipes prefer the loose copy when present. | The launcher/game overlay and site are external repositories and have NOT been migrated. Pack warns that v3 needs those updates; share DS BPS meanwhile. Test Bench deliberately builds legacy v1 packages for the current native game. See exact integration work below. |

2026-11-05:
- Model view: NSBMD texture matrices (scale/rotate/translate) are now applied, so track and object textures tile
  as on the DS. 358 retail materials use one. They are kept when a model is written back.
- Top bar: a **Simple | Studio** switch replaces the hidden Advanced button and is remembered between sessions.
- Model view: the camera stays where the user left it after an edit.
- Track wizard, touch-screen maps: the race's default zoomed map (`local2`/`local3`) is now a top-down picture of
  the new map with the racing line as road, placed by the slot's rectangle-table-2 square. The overview (`global`)
  is drawn in the retail style with the slot's measured transform. Both were blank before.
- **Black floor fixed (race-verified in MKDSrust):** three model-writer bugs made generated course models
  invisible. (1) The model info header was 4 bytes off: position scale 0 and an empty bounding box. (2) Course shapes
  used per-vertex matrix restores, which the game's course drawing doesn't set up. Single-node models are now written
  like retail courses: header position scale plus POSSCALE. (3) The game draws course models 16x (`COURSE_MODEL_SCALE`),
  so the wizard wrote tracks 16x too big. The reader now applies POSSCALE from the right header field, so retail
  models load at true size, and the Track view scales course models 16x to line up with collision and routes. A retail
  Figure-8 model run through our reader and writer renders correctly in game.

An opt-in read-only ROM test changed one menu-model vertex in memory: a **4,578-byte
member patch** versus a **372,728-byte compressed archive**, successfully reapplied
and recompressed. Synthetic tests cover nested archives, missing/unsafe paths,
wrong hashes, reproducibility and combining separate members. No Nintendo bytes
are in the synthetic fixtures or screenshots.

## Exact launcher/game/site work still needed for E

1. In native-builder's shared modpkg.rs, extend Entry with Archive { archive:
   BTreeMap<String, MemberPatch> }; MemberPatch has exactly patch, source_sha256,
   target_sha256. Manifest parsing accepts 1, 2 and 3; archive entries require 3.
   Retain v1/v2 behavior, strict fields and script restrictions. Flatten all member
   patch payload paths when validating/installing ZIP contents; Entry::source()
   must become a multi-source iterator. Keep dependency/load-order handling.
2. In the native game's file-overlay consumer, stage each mod atomically. For an
   archive entry, resolve member names in the current NARC/CARC, decompress leaves,
   check source SHA, BSDIFF, check result SHA, then repack/recompress inside out.
   Reuse toolbox-core/src/archive_delta.rs as the reference algorithm. Keep the
   8-level, 64 MiB per-container/member/output and 256 MiB cumulative bounds. Hash
   mismatches identify same-member conflicts; different members can combine.
   Cache keys must include prior archive bytes and ordered payload/manifest hashes.
   DS export of enabled packages must use this same application path.
3. In site manifest validation and screening, accept the same strict v3 schema,
   path/case/prefix collision checks and flattened payload list. Extend the retail
   HASH-ONLY index with decompressed member paths, sizes and hashes. Validate each
   source hash and BSDIFF output hint; compare the 60% threshold against that
   member's decompressed source size (same 64 KiB exemption). Never compare the
   member patch against the compressed root. Preserve quarantine, add-file retail
   screening, blocked types, ZIP bounds and DS limits. Display member paths in
   conflict/review reports. Integration tests must retain old v1/v2 fixtures plus
   nested/disjoint/same-member v3 cases before enabling uploads or launcher install.

The complete schema is in PLUGIN_FORMAT.md. Community sources studied before each
section are linked in research/STAGE6B_WORKFLOWS.md and credited in CREDITS.md.
No new third-party code was copied; the native-builder repository was read only.

## Verification and run

Passed: cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings;
cargo test --workspace (92 tests, with MKDS_ROM set); app lint, typecheck,
6 Vitest tests, production build and Tauri cargo build; **69 Playwright tests**.
New screenshots are synthetic and cover related pages in both themes. The demo
generator uses fixed synthetic settings instead of local user preferences.
New Playwright flows cover Explorer controls/state, editor/popup routing, pairing,
independent related previews, slot-prefilled replacement, duplicate warnings and
both-theme screenshots. Existing workspace flows remain part of the full suite.
The camera regression test waits for orbit damping to settle before comparing views.

PowerShell, from the repository root:

```powershell
$env:CARGO_HOME = "$PWD/.cache/cargo"
$env:CARGO_NET_OFFLINE = 'true'
# Optional: set MKDS_ROM to the external owned-ROM path in docs/STAGE4.md.
cd app
npm run tauri dev
```

Open your ROM/project, choose Related assets or Game browser, then Replace.
Studio exposes Explorer and the existing expert editors. Save, then Export for DS
outside the repository. Regenerate demos only with cargo run -p toolbox-core
--example demo (always synthetic, independent of MKDS_ROM). For UI tests set
PLAYWRIGHT_BROWSERS_PATH=<repo>/.cache/playwright and MKDSTB_SCREENSHOT_PREVIEW=1,
then run npm run test:e2e from app/. No Git metadata/remotes were changed or pushed.

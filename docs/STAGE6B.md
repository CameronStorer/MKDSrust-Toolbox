# Stage 6B: navigation and related assets (owner's follow-up to STAGE6)

Do this after STAGE6, building on its Game browser. Use `docs/research/ASSET_CATALOG.md` (folder guide, naming, pairing rules, plain labels) as the source for labels and pairing; note its merge rule (palettes in `X.carc`, tiles in `X_us.carc`) and the duplicate KartModelMenu folder vs archive. Same rules: breadth first, checks green, ROM read in place,
changes left uncommitted.

## A. Explorer (the Advanced file view stays, and gets easier)
- **Collapse all** and **Expand all** buttons.
- **Sort and group by type** while keeping the folder hierarchy:
  - archives (.carc/.narc), images, palettes, models, textures, animations, collision, track data, text, audio, code;
  - each type has an icon and a friendly name ("Model", "Palette", "Track layout").
- Folder hints: "contains 13 character models", "Mario's karts".
- **Language toggle:** hide or show the es/fr/ge/it files (`_es`, `_fr`, `_ge`, `_it`), defaulting to English (US).
- **Keep the tree's scroll position, expansion and selection** when switching tabs or leaving and returning.
- **Double-click opens the right editor** for the file type. For types without an editor, show a preview popup
  with Export.
- **Every file name is a link.** History lines, validation issues, export reports, wizard reports and warnings open
  the file in the Explorer and its editor.

## B. Automatic pairing
- **Palettes with images:** NCLR with NCGR, NSCR and NCER, using naming conventions, the same archive, size
  compatibility and the research tables. Record the result in `mkds-data` so it isn't guessed again. Show the
  pairing and allow an override (Advanced).
- **Models with textures and animations:**
  - `X.nsbmd` with `X.nsbtx`;
  - `P_XX` with `P_XX_{drive,spin,win,lose}.nsbca` and `P_faceanim.nsbtp`;
  - karts with tires, emblems and shadows;
  - track models with their textures.

  The Model view shows textures and plays animations automatically, with an animation picker.

## C. Related assets page
One page per character, kart and track, with every related asset in its own viewport. Each says in one line when
the game uses it.
- **Character:**
  - race model (animated, with clip buttons);
  - far-away model;
  - menu model;
  - icons;
  - voice (play button);
  - their karts.
- **Kart:**
  - body in the race and menu;
  - tires (S/M/L) at the wheel positions from `kartoffsetdata.bin`;
  - shadow and emblem;
  - stats.
- **Track:**
  - course model;
  - sky;
  - collision overlay;
  - minimap;
  - routes and objects;
  - music.
- "Replace…" buttons open the matching wizard, already set to that slot.

## D. Done means
Playwright flows for each item on synthetic data, screenshots, and STATUS.md updated.

## E. Small mod packages (owner hit this)
Packing a project that changed one model in `data/KartModelMenu.carc` warned "patch is at least 60% of the target
size; manual review required". The cause: packages diff the compressed archive, and any change reshuffles the whole
LZ stream.
- Diff archive members **decompressed**: one patch per changed member, applied by the launcher, which recompresses.
  This needs a package-format bump that is backwards compatible, documented in docs/PLUGIN_FORMAT.md, and mirrored
  in the launcher and site screening. If that can't be done here, write the exact launcher and site changes needed
  in STATUS.md.
- Warn when editing a file that has a duplicate the game doesn't read: `KartModelMenu.carc` vs the loose
  `KartModelMenu/` folder (see ASSET_CATALOG.md). Edit the copy the game uses by default.

## F. Linked variants (owner idea)
A character or kart model exists in several copies the player sees at different times:
- race model `P_XX` (KartModelSub);
- menu model `P_XX` (KartModelMenu, the copy the game reads);
- far-away model `E_XX` (KartModelMainA/B);
- the kart's menu and race copies and its shadow.

So:
- The Model view shows a **"Linked copies"** strip listing each variant with its role and polygon budget, plus an
  **"Apply to all copies"** toggle (on by default for beginners).
- **Transforms and edits** (move, rotate, scale, delete part, cut) apply to every linked copy in one undoable step.
- **Import over all copies:** one import replaces every variant. Each copy is decimated to its own measured budget
  (the far model much lower), keeps its required structure (P_: the base skeleton and clips via the character-wizard
  rig; E_: one static node), and the result shows the triangle count per copy.
- Skip the unused developer archive `KartModelMain.carc` and the stale `KartModelMenu.carc` duplicate (see
  ASSET_CATALOG.md), or mark them clearly.

## G. Clickable model contents (Outliner)
- **Bones:** clicking one highlights it in the viewport (gizmo at the joint, its vertices tinted) and selects it in
  the viewport too. Show the parent chain, which animation clips move it, and the vertex count assigned to it. A
  "Paint vertices to this bone" action reuses the wizard's rig brush.
- **Materials and textures:** clicking one opens a popup showing the texture with its palette (zoomable). Offer
  Replace image (PNG import with auto-conversion), Recolour, Export PNG, and "Highlight on model" (tint the
  triangles using it).

## H. Model editing like Blender (owner requests)
- **Bug, do first:** the camera resets after every model action (transform, decimate, cut, delete...). Keep the
  camera exactly where the user left it. Only "Frame selected (F)" and opening a different model move it.
- **Multi-select parts:** Shift/Ctrl-click in the viewport and the Outliner, box select, and select all/none.
  Actions apply to every selected part.
- **Edit mode (Tab)** with vertex, edge and face selection (keys 1/2/3, Shift to add, box select, select linked):
  - **G** to grab or move, **R** rotate, **S** scale, with X/Y/Z axis locks, typed values and snapping;
  - **E** to extrude faces or edges (push in or out along the normal);
  - **I** to inset;
  - delete vertices, edges or faces;
  - merge selected;
  - live triangle count and DS budget while editing.

  Every operation is one undo step in the project history and keeps the model DS-valid (rigged models keep
  per-vertex bone assignments; new vertices inherit from their neighbours).

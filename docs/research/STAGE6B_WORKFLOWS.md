# Stage 6B Run 1 workflow references (2026-11-05)

## Run 2: linked models, contents, mesh editing

Sources read before implementation (workflow/format study; no third-party code copied):

- [NitroPaint nsbtxviewer.c](https://github.com/Garhoogin/NitroPaint/blob/main/NitroPaint/editor/nsbtxviewer.c)
  and [textureeditor.c](https://github.com/Garhoogin/NitroPaint/blob/main/NitroPaint/editor/textureeditor.c)
  (BSD-2-Clause): independently selected texture and palette resources, nearest-pixel zoom,
  palette swatches, image replacement through format conversion, and PNG export. The Outliner
  popup uses the material's actual named resources; shared palette edits are explained there.
- [MKTB G3dModelManager.cs](https://github.com/HaroohiePals/MarioKartToolbox/blob/develop/src/HaroohiePals.Nitro.NitroSystem/G3d/G3dModelManager.cs)
  (MIT), `BindTextures`: resolve material-to-texture and material-to-palette dictionaries separately.
  An absent texture stays explicitly unbound. Direct-colour textures have no palette controls.
- [apicula skeleton](https://github.com/scurest/apicula/blob/master/src/skeleton/mod.rs)
  and [vertex_record.rs](https://github.com/scurest/apicula/blob/master/src/skeleton/vertex_record.rs)
  (0BSD): joint parent transforms and per-vertex matrix/weight influences are separate from mesh parts.
  Bone inspection counts nonzero influences, displays the parent chain and joint tracks, and reuses
  the existing rigid rig brush. New mesh vertices copy all source attributes, including skin weights.
- [Blender default keymap](https://github.com/blender/blender/blob/main/scripts/presets/keyconfig/keymap_data/blender_default.py)
  (GPL-3.0-or-later): Tab, 1/2/3, A/Alt+A, B, L, G/R/S, E, I, X/Delete and M.
  Modal transforms use axis constraints, numeric values, Ctrl increments, Shift precision,
  Enter/LMB confirmation and Escape/RMB cancellation. X and M open explicit operation menus.
  E extrudes a region with boundary walls; I insets a region rather than each triangle separately.

F's associations follow ASSET_CATALOG's runtime copies. Source geometry is never copied into a linked
variant for ordinary edits: each retains its mesh, materials and rig. Part subsets require a unique
matching part name; element subsets map by nearest bind-space source vertex, never by index across LODs.
All-copy selection maps to every destination part. Ambiguous subsets fail atomically with a message to
edit the variant alone. Imports use each copy's clean-ROM polygon/vertex budget and its existing rig;
E variants get one identity node; shadows stay flat. The existing conservative reducer runs first;
the LOD fallback permits a two-triangle patch to become one without changing track reduction behavior.

All mutations stage in memory and record one Project::edit_many command. NSBMD serialization remains
nitro_formats::model_write, with its verified header/POSSCALE and course scale behavior unchanged.

## Run 1

Structure and workflow observations only. No community code was copied in this run.

| Section | Source studied before implementation | Applied here |
|---|---|---|
| A | [Tinke Sistema.cs](https://github.com/pleonex/tinke/blob/master/Tinke/Sistema.cs), `treeSystem_AfterSelect`, `treeSystem_MouseDoubleClick`, expand-all handling | Keep nested filesystem identity; select for inspection and activate by detected type. |
| A | [EveryFileExplorer FileBrowser.cs](https://github.com/Gericom/EveryFileExplorer/blob/master/LibEveryFileExplorer/UI/FileBrowser.cs), `UpdateDirectories`, `SetExpandState`, `UpdateContent`, `listView1_ItemActivate` | Group immediate siblings by category with icons; preserve selection, expansion and scroll outside the mounted view. |
| B | [NitroPaint combo2d.c](https://github.com/Garhoogin/NitroPaint/blob/main/NitroPaint/object/combo2d.c), `combo2dLink`, format requirements; [ncgrviewer.c](https://github.com/Garhoogin/NitroPaint/blob/main/NitroPaint/editor/ncgrviewer.c), associated palette viewer | Explicit linked palette/tiles/screen/cell resources, manual override, shared preview/edit inputs. Our mkds-data rules add the game's naming and neutral/localized merge, with depth/index checks. |
| B | [MKTB G3dModelManager.cs](https://github.com/HaroohiePals/MarioKartToolbox/blob/develop/src/HaroohiePals.Nitro.NitroSystem/G3d/G3dModelManager.cs), `BindTextures`, animation texture resources | Match texture resources by name and keep a model's own clips. Never pick an arbitrary sibling texture or animation. |
| C | [MKTB MkdsBinaryCourse.cs](https://github.com/HaroohiePals/MarioKartToolbox/blob/develop/src/HaroohiePals.NitroKart/Course/MkdsBinaryCourse.cs), `MainArchive`, `TexArchive`, `LoadMapGraphics`, map/collision updates | A course page combines model, sky, collision, NKM routes/objects and the Tex archive's explicit local minimap pairing. Cards have independent view/animation state. |
| E | Read-only local `mkds-native-builder/1_build_tool/src/modpkg.rs`: `Entry`, `Manifest::parse`, `Package::validate`, `make_patch`; existing Toolbox NARC rebuilding and LZ77 splice code | BSDIFF40 per decompressed member and rebuild/recompress on consumption. The inspected builder currently supports whole-file v1/v2 entries, so v3 needs the documented external migration. No changes made to that repository. |

Labels, character/kart IDs, duplicate preference, tire/seat offsets, and voice bank mapping use
ASSET_CATALOG.md, CHARACTERS_KARTS.md and the existing Stage 5 implementation. Inferred icon-cell order is labelled
as inferred. No guessed palette is chosen merely because it is nearby. Unresolved roles are displayed explicitly.

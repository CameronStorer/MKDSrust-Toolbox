# Stage 6B Run 1 workflow references (2026-11-05)

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

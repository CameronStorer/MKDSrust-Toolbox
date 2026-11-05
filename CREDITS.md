# Credits

Ideas, UX conventions and format knowledge come from the MKDS/DS modding community. See docs/PRIOR_ART.md.
Any code ported from another project is listed here with its license, and keeps its original header.

| Project | License | Used for |
|---|---|---|
| [MKDSrust builder](https://github.com/CameronStorer/MKDSrust) | GPL-3.0 | LZ77 / NARC / mod packing (same author) |
| [apicula](https://github.com/scurest/apicula) | 0BSD | Reference for model/GX, joint animation, TEX0 and texture dictionary layouts; independent bounded implementation |
| [Mario Kart Toolbox](https://github.com/HaroohiePals/MarioKartToolbox) (Haroohie Pals) | MIT | NKM/KCL and collision attribute layouts, Nitro resource dictionary reference; independent Rust implementations |
| [ndspy](https://github.com/RoadrunnerWMC/ndspy) (RoadrunnerWMC) | GPL-3.0-or-later | SDAT/SSEQ/SBNK/SWAR/SWAV/SSAR/STRM format references; original Rust codecs and writers, no source code copied |
| [jpeg-decoder](https://github.com/image-rs/jpeg-decoder) (image-rs Developers) | MIT OR Apache-2.0 | Pure Rust JPEG texture import, default features disabled |
| [DS Pokémon Hacking format research](https://ds-pokemon-hacking.github.io/docs/universal/resources/nitro/graphics_2d/section_cebk/) (Gonhex, NOCASH, magical) | Format reference only | NCER/CEBK and NANR/ABNK layouts; independent implementation, no code copied |
| [dockview](https://github.com/mathuo/dockview) | MIT | Dockable workspace panels |
| [Inter](https://github.com/rsms/inter) | SIL OFL 1.1 | Bundled UI font, via Fontsource; license in `app/public/licenses/Inter-OFL.txt` |
| [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) | SIL OFL 1.1 | Bundled hex/numeric font, via Fontsource; license in `app/public/licenses/JetBrainsMono-OFL.txt` |
| [Lucide](https://lucide.dev) | ISC | UI icons (`lucide-react`) |
| [qbsdiff](https://github.com/hucsmn/qbsdiff) | MIT | BSDIFF40 patch creation and test application |
| [Tinke](https://github.com/pleonex/tinke) | GPL-3.0 | Explorer workflow reference: typed file activation, nested archives and expand-all; independent implementation, no code copied |
| [EveryFileExplorer](https://github.com/Gericom/EveryFileExplorer) | GPL-3.0 | Explorer workflow reference: categories/icons and retained expansion/selection; independent implementation, no code copied |
| [NitroPaint](https://github.com/Garhoogin/NitroPaint) | BSD-2-Clause | Linked palette/character/screen workflow reference; independent catalog-based association rules, no code copied |

`crates/nitro-formats/src/lz77.rs` preserves the original builder module header and carries a provenance/SPDX header.
The KCL header check in `detect.rs` is also ported from that module. Archive parsing and writing use one bounded
implementation in `archive.rs`; unused builder format tables and duplicate archive helpers have been removed.
No game assets, SDK code or extracted data are included. Demo JSON, image and screenshots are generated from
`toolbox-core::synthetic`; the Toolbox application icon is original geometric artwork.

Stage 6B's model association and related-course views also reference MKTB's `G3dModelManager` and `MkdsBinaryCourse`
(MIT): named texture resources, separate main/Tex archives, and explicit local/global minimap combinations.
The sources studied and the resulting choices are recorded in `docs/research/STAGE6B_WORKFLOWS.md`.

Stage 6B Run 2 also studies NitroPaint's texture/palette editors (BSD-2-Clause),
apicula's skeleton/vertex influence representation (0BSD), and
[Blender's default keymap](https://github.com/blender/blender/blob/main/scripts/presets/keyconfig/keymap_data/blender_default.py)
(GPL-3.0-or-later) for modal mesh-editing behavior. These are workflow references;
the linked-copy commands, mesh operations and UI are original implementations. No source code was copied.

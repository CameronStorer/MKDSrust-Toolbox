# Credits

Ideas, UX conventions and format knowledge come from the MKDS/DS modding community. See docs/PRIOR_ART.md.
Any code ported from another project is listed here with its license, and keeps its original header.

| Project | License | Used for |
|---|---|---|
| [MKDSrust builder](https://github.com/CameronStorer/MKDSrust) | GPL-3.0 | LZ77 / NARC / mod packing (same author) |
| [apicula](https://github.com/scurest/apicula) | 0BSD | Reference for model/GX, joint animation, TEX0 and texture dictionary layouts; independent bounded implementation |
| [DS Pokémon Hacking format research](https://ds-pokemon-hacking.github.io/docs/universal/resources/nitro/graphics_2d/section_cebk/) (Gonhex, NOCASH, magical) | Format reference only | NCER/CEBK and NANR/ABNK layouts; independent implementation, no code copied |
| [dockview](https://github.com/mathuo/dockview) | MIT | Dockable workspace panels |
| [Inter](https://github.com/rsms/inter) | SIL OFL 1.1 | Bundled UI font, via Fontsource; license in `app/public/licenses/Inter-OFL.txt` |
| [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) | SIL OFL 1.1 | Bundled hex/numeric font, via Fontsource; license in `app/public/licenses/JetBrainsMono-OFL.txt` |
| [Lucide](https://lucide.dev) | ISC | UI icons (`lucide-react`) |
| [qbsdiff](https://github.com/hucsmn/qbsdiff) | MIT | BSDIFF40 patch creation and test application |

`crates/nitro-formats/src/lz77.rs` preserves the original builder module header and carries a provenance/SPDX header.
The KCL header check in `detect.rs` is also ported from that module. Archive parsing and writing use one bounded
implementation in `archive.rs`; unused builder format tables and duplicate archive helpers have been removed.
No game assets, SDK code or extracted data are included. Demo JSON, image and screenshots are generated from
`toolbox-core::synthetic`; the Toolbox application icon is original geometric artwork.

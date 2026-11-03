# Prior art: the current MKDS modding toolchain

Surveyed 2026-11-03 from the community's recommended-tools list. The point is to learn from these tools and stay
interoperable with them, not to copy their code. Copying code is only allowed when the license is
GPL-3.0-compatible, it is credited in `CREDITS.md`, and the file keeps its original license header.

| Tool | What it does | Tech | License | What we take from it |
|---|---|---|---|---|
| [Mario Kart Toolbox](https://github.com/HaroohiePals/MarioKartToolbox) (HaroohiePals) | Course editor: every NKM section, error detection, KCL generator/viewer, 2D/3D gizmos, live camera preview | C#, Dear ImGui, OpenGL | MIT | The bar for the Track workspace. Its library split (`IO`/`Nitro`/`NitroSystem.G3d`/`KCollision`/`MarioKart.Validation`/`Actions`) is a good model for our crates. Its "Validation" (error detection) is the model for our DS checks. MIT, so its NKM/KCL logic may be ported with credit. |
| [NitroPaint](https://github.com/Garhoogin/NitroPaint) | Palettes, tiles, screens, cells, animations, textures (NCLR/NCGR/NSCR/NCER/NANR/NSBTX), with high-quality colour reduction | C, Win32 | BSD-2-Clause | The bar for the Texture/2D workspace, especially colour quantization and texture-format conversion (4x4 compressed, a3i5, a5i3). |
| [NNS Blender plugin](https://github.com/jellees/nns-blender-plugin) | Blender 2.8+ exporter to Nitro *intermediate* files (.imd/.ica/.ita/.itp), which g3dcvtr then turns into .nsbmd | Python | none stated | Shows what a DS-ready model needs: materials, polygon attributes, and matrices per bone. We must not copy the code (no license). Our models must be able to round-trip with Blender either way. |
| [nitrog3d](https://github.com/Fexty12573/nitrog3d) | Blender extension that reads and writes NSBMD directly | Python | MIT | Proves you can write NSBMD directly without g3dcvtr. Its encoder can be referenced/ported with credit. |
| G3DCVTR / Ass2IMD | Nintendo SDK converter (imd → nsbmd); Assimp → imd | closed / SDK | proprietary SDK | **Never bundled or required.** Our exporter writes NSBMD itself. |
| [apicula](https://github.com/scurest/apicula) | Rip, view and convert NSBMD/NSBCA/NSBTP to COLLADA/glTF | Rust | 0BSD | Same language as our core. Its NSBMD/NSBCA decoding can be used directly (0BSD), and it's our reference for nsbmd → glTF. |
| [Nitro Studio 2 (Deluxe fork)](https://github.com/PlatinumLucario/NitroStudio2Deluxe) | SDAT editor: SSEQ/SBNK/SWAR/STRM, MIDI/SF2/DLS export | C# | check before use | The bar for the Audio workspace. |
| wav2swav / swav2wav, smfconv / seqconv | Sample and sequence conversion (seqconv is SDK) | various | mixed | Our Audio workspace does MIDI ↔ SSEQ and WAV ↔ SWAV itself. |
| [NitroEffectMaker](https://github.com/HaroohiePals/NitroEffectMaker) | Editor for the SPL particle library | C# | MIT | Future Effects workspace. |
| JLayout | Editor for the game's 2D UI layouts | — | unknown | Future UI/2D workspace. |
| [MKDS-decomp-headers](https://github.com/HaroohiePals/MKDS-decomp-headers), [mkds-re](https://github.com/XorTroll/mkds-re), RuntimeCodeModules, Ermii's devkitPro folder | ASM/C code injection for real DS builds | C/ASM | headers: none stated; mkds-re: MIT | The MKDSrust equivalent is Lua scripts and Rust source patches. A future "DS export" may target these code-injection setups. |
| NSMBE fork, Tinke, EveryFileExplorer | General ROM/archive explorers | C# | GPL-3 / GPL-3 / GPL-3 | ROM explorer behaviour (FNT/FAT, NARC, nested archives). |

## What today's workflow looks like (and what's painful)
- **Models:** Blender, then the NNS plugin writes .imd, then the SDK's g3dcvtr makes .nsbmd. Separately, NitroPaint
  prepares textures, then you hex-edit or use an archive tool to put the file into the ROM, then test in an emulator.
  That's four or more tools, and some steps need a leaked or proprietary SDK binary.
- **Tracks:** Blender for the model, then KCL from an OBJ (MKTB), then NKM objects/paths in MKTB, then textures in
  NitroPaint, then repack the CARC, then test in an emulator.
- **Audio:** Nitro Studio, plus command-line converters.
- **Code:** C/ASM injection with devkitPro, linker scripts and symbol files.

Our goal: **one app, one project, one Play button**. And everything exports to open formats (glTF, PNG, MIDI, WAV,
OBJ) so nobody is locked in.

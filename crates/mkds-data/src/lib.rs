pub const GAME_CODE: &str = "AMCE";
pub const TEXTURE_VRAM: usize = 512 * 1024;
pub const PALETTE_VRAM: usize = 96 * 1024;
pub const MAX_ROM: usize = 256 * 1024 * 1024;
pub const MAX_MESSAGE_UNITS: usize = 1024;
pub const ARM9_BASE: u32 = 0x0200_0000;
pub const CUP_TABLE: u32 = 0x0215_4128;
pub const COURSE_TABLE: u32 = 0x0215_4228;
pub const MUSIC_TABLE: u32 = 0x0215_316c;
pub const MUSIC_BRANCHES: u32 = 0x0202_554c;
pub const SOUND_HEAP: u32 = 716_800;
pub const SAMPLE_TARGET: u32 = 250 * 1024;
pub const CHARACTERS: &[(&str, &str)] = &[
    ("MR", "mario"),
    ("DK", "donkey"),
    ("KO", "kinopio"),
    ("KP", "koopa"),
    ("PC", "peach"),
    ("WR", "wario"),
    ("YS", "yoshi"),
    ("LG", "luigi"),
    ("KA", "karon"),
    ("DS", "daisy"),
    ("WL", "waluigi"),
    ("RB", "robo"),
    ("HH", "heyho"),
];
/// MKDS attributes: type occupies bits 8–12; wall/floor are separate flags.
/// Layout reference: Mario Kart Toolbox (MIT). Values are format facts.
pub const fn collision_type(attribute: u16) -> u16 {
    (attribute >> 8) & 31
}
pub const fn collision_attribute(kind: u16, wall: bool, floor: bool) -> u16 {
    (kind & 31) << 8 | if wall { 1 << 14 } else { 0 } | if floor { 1 << 15 } else { 0 }
}
/// Observed type ids from the local opt-in retail audit; no game bytes.
pub const RETAIL_COLLISION_TYPES: &[u16] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
];
/// The game draws `course_model.nsbmd` at 16x its model units (collision and map data use the scaled units;
/// measured: Mario Circuit model 270.09 x 16 = collision 4321.99).
pub const COURSE_MODEL_SCALE: f32 = 16.;
pub const COLLISION_TYPES: &[(u16, &str)] = &[
    (0, "Road"),
    (1, "Slippery road"),
    (2, "Weak offroad"),
    (3, "Offroad"),
    (4, "Sound trigger"),
    (5, "Heavy offroad"),
    (6, "Slippery road 2"),
    (7, "Boost pad"),
    (8, "Wall"),
    (9, "Invisible wall"),
    (10, "Out of bounds"),
    (11, "Fall boundary"),
    (12, "Jump pad"),
    (13, "Road without drivers"),
    (14, "Wall without drivers"),
    (15, "Cannon"),
    (16, "Edge wall"),
    (17, "Water fall"),
    (18, "Minimum speed boost"),
    (19, "Loop"),
    (20, "Special road"),
    (21, "Wall 3"),
    (22, "Recalculate route"),
];

/// Player-facing character labels, in slot order.
pub const CHARACTER_NAMES: &[&str] = &[
    "Mario",
    "Donkey Kong",
    "Toad",
    "Bowser",
    "Peach",
    "Wario",
    "Yoshi",
    "Luigi",
    "Dry Bones",
    "Daisy",
    "Waluigi",
    "R.O.B.",
    "Shy Guy",
];
/// Menu/scene sequence ids confirmed in docs/research/TRACKS_AUDIO_CUPS.md.
pub const SCENE_MUSIC: &[(usize, &str)] = &[
    (1, "Single-player menu"),
    (2, "Multiplayer menu"),
    (3, "Wi-Fi menu"),
    (6, "Records menu"),
    (8, "Grand Prix flyover 1"),
    (9, "Grand Prix flyover 2"),
    (10, "Grand Prix flyover 3"),
    (13, "Battle flyover"),
    (14, "Boss intro"),
    (43, "Battle theme"),
    (45, "Boss battle"),
    (63, "Reward"),
    (66, "Grand Prix results"),
    (69, "Credits 1"),
    (70, "Credits 2"),
    (75, "Intro"),
];
pub mod assets;

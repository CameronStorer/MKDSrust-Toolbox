pub const GAME_CODE: &str = "AMCE";
pub const TEXTURE_VRAM: usize = 512 * 1024;
pub const PALETTE_VRAM: usize = 96 * 1024;
pub const MAX_ROM: usize = 256 * 1024 * 1024;
pub const MAX_MESSAGE_UNITS: usize = 1024;
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

use vek::Extent2;

/// Returns `true` if all clients of that version should have that mapres shipped with their client.
pub fn is_external_name(name: &str) -> bool {
    matches!(
        name,
        "bg_cloud1"
            | "bg_cloud2"
            | "bg_cloud3"
            | "desert_doodads"
            | "desert_main"
            | "desert_mountains"
            | "desert_mountains2"
            | "desert_sun"
            | "easter"
            | "generic_deathtiles"
            | "generic_lamps"
            | "generic_shadows"
            | "generic_unhookable"
            | "grass_doodads"
            | "grass_main"
            | "jungle_background"
            | "jungle_deathtiles"
            | "jungle_doodads"
            | "jungle_main"
            | "jungle_midground"
            | "jungle_unhookables"
            | "light"
            | "moon"
            | "mountains"
            | "snow"
            | "stars"
            | "sun"
            | "winter_doodads"
            | "winter_main"
            | "winter_mountains"
            | "winter_mountains2"
            | "winter_mountains3"
    )
}

/// Returns the opaque table for external images.
/// None, if the given name is not a valid external image name, or if the image not square.
/// The latter restriction is taken from the client source, don't ask me.
pub fn external_opaque_table(name: &str) -> Option<[[bool; 16]; 16]> {
    Some(match name {
        "desert_doodads" => DESERT_DOODADS,
        "desert_main" => DESERT_MAIN,
        "desert_sun" => DESERT_SUN,
        "easter" => EASTER,
        "generic_deathtiles" => GENERIC_DEATHTILES,
        "generic_lamps" => GENERIC_LAMPS,
        "generic_shadows" => GENERIC_SHADOWS,
        "generic_unhookable" => GENERIC_UNHOOKABLE,
        "grass_doodads" => GRASS_DOODADS,
        "grass_main" => GRASS_MAIN,
        "jungle_deathtiles" => JUNGLE_DEATHTILES,
        "jungle_doodads" => JUNGLE_DOODADS,
        "jungle_main" => JUNGLE_MAIN,
        "jungle_midground" => JUNGLE_MIDGROUND,
        "jungle_unhookables" => JUNGLE_UNHOOKABLES,
        "light" => LIGHT,
        "moon" => MOON,
        "snow" => SNOW,
        "sun" => SUN,
        "winter_doodads" => WINTER_DOODADS,
        "winter_main" => WINTER_MAIN,
        _ => return None,
    })
}

/// Returns the (width, height) of external images.
/// None, if the given name is not a valid external image name.
pub fn external_dimensions(name: &str) -> Option<Extent2<u32>> {
    Some(match name {
        "bg_cloud1" => Extent2::new(2048, 1024),
        "bg_cloud2" => Extent2::new(2048, 1024),
        "bg_cloud3" => Extent2::new(1024, 512),
        "desert_doodads" => Extent2::new(1024, 1024),
        "desert_main" => Extent2::new(1024, 1024),
        "desert_mountains" => Extent2::new(1024, 512),
        "desert_mountains2" => Extent2::new(1024, 512),
        "desert_sun" => Extent2::new(1024, 1024),
        "easter" => Extent2::new(1024, 1024),
        "generic_deathtiles" => Extent2::new(1024, 1024),
        "generic_lamps" => Extent2::new(1024, 1024),
        "generic_shadows" => Extent2::new(1024, 1024),
        "generic_unhookable" => Extent2::new(1024, 1024),
        "grass_doodads" => Extent2::new(1024, 1024),
        "grass_main" => Extent2::new(1024, 1024),
        "jungle_background" => Extent2::new(809, 1312),
        "jungle_deathtiles" => Extent2::new(1024, 1024),
        "jungle_doodads" => Extent2::new(1024, 1024),
        "jungle_main" => Extent2::new(1024, 1024),
        "jungle_midground" => Extent2::new(1024, 1024),
        "jungle_unhookables" => Extent2::new(1024, 1024),
        "light" => Extent2::new(256, 256),
        "moon" => Extent2::new(1024, 1024),
        "mountains" => Extent2::new(1024, 512),
        "snow" => Extent2::new(64, 64),
        "stars" => Extent2::new(265, 128),
        "sun" => Extent2::new(512, 512),
        "winter_doodads" => Extent2::new(1024, 1024),
        "winter_main" => Extent2::new(1024, 1024),
        "winter_mountains" => Extent2::new(1024, 512),
        "winter_mountains2" => Extent2::new(1024, 512),
        "winter_mountains3" => Extent2::new(1024, 512),
        _ => return None,
    })
}

#[rustfmt::skip]
const DESERT_DOODADS: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false,  true, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const DESERT_MAIN: [[bool; 16]; 16] = [
    [false,  true,  true,  true, false,  true,  true,  true, false, false,  true,  true, false, false,  true, false],
    [ true, false,  true, false,  true,  true, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false,  true,  true,  true, false,  true, false, false, false, false, false, false],
    [ true,  true,  true,  true,  true,  true, false, false, false, false, false, false, false,  true, false, false],
    [ true,  true,  true,  true, false,  true,  true, false, false, false, false, false, false,  true, false, false],
    [false,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false, false, false],
    [ true, false,  true, false,  true,  true,  true,  true,  true,  true, false, false, false, false, false, false],
    [false, false, false, false,  true,  true,  true,  true,  true,  true, false, false, false, false, false, false],
    [ true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false, false, false],
    [ true,  true, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const DESERT_SUN: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false,  true,  true,  true,  true, false, false, false, false, false, false],
    [false, false, false, false, false,  true,  true,  true,  true,  true,  true, false, false, false, false, false],
    [false, false, false, false,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false],
    [false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false],
    [false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false],
    [false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false, false, false, false,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false],
    [false, false, false, false, false,  true,  true,  true,  true,  true,  true, false, false, false, false, false],
    [false, false, false, false, false, false,  true,  true,  true,  true, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const EASTER: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const GENERIC_DEATHTILES: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const GENERIC_LAMPS: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const GENERIC_SHADOWS: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const GENERIC_UNHOOKABLE: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false,  true, false, false, false, false, false, false,  true, false, false, false, false],
    [false, false, false,  true,  true, false, false, false, false, false,  true,  true, false, false, false, false],
    [false,  true, false, false,  true, false, false, false,  true, false, false,  true, false, false, false, false],
    [false,  true, false, false,  true, false, false, false,  true, false, false,  true, false, false, false, false],
    [ true,  true,  true, false,  true, false, false,  true,  true,  true, false,  true, false, false, false, false],
    [false,  true, false, false,  true, false, false, false,  true, false, false,  true, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false,  true, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false,  true,  true, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true, false, false,  true, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true, false, false,  true, false, false, false, false, false, false, false, false, false, false, false],
    [ true,  true,  true, false,  true, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true, false, false,  true, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const GRASS_DOODADS: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false,  true,  true,  true, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true, false, false, false, false, false,  true, false, false, false,  true, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false,  true,  true, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true, false, false, false, false, false, false,  true, false, false, false, false, false],
    [false, false,  true, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const GRASS_MAIN: [[bool; 16]; 16] = [
    [false,  true,  true,  true, false, false, false, false, false, false, false,  true,  true,  true,  true,  true],
    [ true,  true,  true,  true, false, false, false, false, false, false, false,  true,  true,  true,  true,  true],
    [false, false, false, false, false, false, false, false, false, false,  true,  true,  true,  true,  true,  true],
    [ true,  true,  true,  true, false,  true,  true, false, false, false, false, false, false,  true,  true,  true],
    [ true,  true,  true,  true,  true,  true,  true, false,  true,  true,  true, false, false, false, false, false],
    [ true,  true, false,  true, false,  true,  true, false,  true,  true,  true, false, false, false, false, false],
    [false, false,  true,  true, false, false, false, false,  true,  true,  true, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true, false,  true,  true,  true,  true, false, false, false, false, false, false, false, false, false],
    [false,  true, false,  true,  true,  true,  true, false, false, false, false, false, false, false, false, false],
    [false, false, false,  true,  true,  true,  true, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const JUNGLE_DEATHTILES: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const JUNGLE_DOODADS: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false,  true,  true,  true,  true,  true,  true,  true, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false,  true, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false,  true,  true, false, false,  true,  true, false],
    [false, false, false, false, false, false, false, false, false,  true, false, false, false,  true, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false,  true,  true, false, false, false, false,  true,  true, false, false, false, false, false, false],
];

#[rustfmt::skip]
const JUNGLE_MAIN: [[bool; 16]; 16] = [
    [false,  true,  true,  true, false, false, false, false, false, false, false, false, false,  true, false, false],
    [ true,  true,  true,  true, false, false, false, false, false, false, false, false, false,  true, false, false],
    [false, false, false, false, false, false, false, false, false, false,  true,  true,  true,  true, false, false],
    [ true,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false, false],
    [ true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false],
    [ true,  true, false, false, false, false, false,  true,  true,  true,  true,  true, false, false, false, false],
    [false,  true,  true, false, false, false, false, false,  true,  true,  true, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true, false,  true, false,  true,  true,  true,  true,  true, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const JUNGLE_MIDGROUND: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false,  true, false, false, false, false, false, false, false, false, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true, false, false, false, false, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false, false],
    [false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false, false,  true, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false,  true, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false,  true, false, false,  true, false, false, false, false, false, false, false, false, false],
    [false, false, false,  true, false, false,  true,  true, false, false, false,  true, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true,  true,  true, false, false, false, false, false, false, false, false, false, false],
    [false,  true, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false,  true, false,  true, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const JUNGLE_UNHOOKABLES: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const LIGHT: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const MOON: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false,  true,  true,  true,  true, false, false, false, false, false, false],
    [false, false, false, false, false,  true,  true,  true,  true,  true,  true, false, false, false, false, false],
    [false, false, false, false,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false],
    [false, false, false, false,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false],
    [false, false, false, false, false,  true,  true,  true,  true,  true,  true, false, false, false, false, false],
    [false, false, false, false, false, false,  true,  true,  true,  true, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const SNOW: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const SUN: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false,  true,  true,  true,  true,  true,  true, false, false, false, false, false],
    [false, false, false, false,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false, false, false,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false],
    [false, false, false, false,  true,  true,  true,  true,  true,  true,  true,  true, false, false, false, false],
    [false, false, false, false, false,  true,  true,  true,  true,  true,  true, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

#[rustfmt::skip]
const WINTER_DOODADS: [[bool; 16]; 16] = [
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false,  true, false, false, false, false, false,  true, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false, false],
    [false,  true,  true,  true, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false,  true, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false,  true, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false,  true, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false,  true,  true,  true, false, false, false],
];

#[rustfmt::skip]
const WINTER_MAIN: [[bool; 16]; 16] = [
    [false,  true, false,  true,  true, false, false, false, false,  true,  true,  true,  true,  true,  true,  true],
    [false,  true,  true,  true, false, false,  true,  true, false,  true,  true,  true,  true,  true,  true,  true],
    [false,  true,  true,  true,  true,  true,  true,  true,  true, false,  true,  true,  true,  true,  true, false],
    [false,  true,  true,  true,  true,  true,  true,  true,  true, false,  true,  true, false, false,  true, false],
    [false,  true,  true,  true,  true,  true,  true,  true,  true, false,  true, false, false, false,  true, false],
    [false, false,  true,  true,  true, false, false, false,  true,  true,  true, false,  true,  true, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [ true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false,  true, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false,  true, false, false],
    [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false],
];

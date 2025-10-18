use twmap::*;

use fixed::traits::{Fixed, ToFixed};
use fixed::types::{I17F15, I22F10, I27F5};
use image::RgbaImage;
use ndarray::arr2;
use vek::{Disk, Extent2, Rect, Rgba, Vec2};

use std::fs;

fn map_from_path(path: &str) -> TwMap {
    let data = fs::read(path).unwrap();
    TwMap::parse(&data).unwrap()
}

fn map_load_save_load_save_eq(path: &str, version: Version) {
    let mut map = map_from_path(path);
    assert_eq!(map.version, version);

    map_save_load_save_eq(&mut map);
}

fn map_save_load_save_eq(original_map: &mut TwMap) {
    original_map.load().unwrap();

    let mut first_saved_data = Vec::new();
    original_map.save(&mut first_saved_data).unwrap();
    let mut loaded_map = TwMap::parse(&first_saved_data).unwrap();
    loaded_map.load().unwrap();
    assert_eq!(loaded_map, *original_map);

    let mut second_saved_data = Vec::new();
    loaded_map.save(&mut second_saved_data).unwrap();
    assert_eq!(second_saved_data, first_saved_data);
}

fn generate_data(count: usize, step: u8, reverse: bool) -> Vec<u8> {
    let mut data = Vec::new();
    let mut value: u8 = 0;
    for _ in 0..count {
        data.push(value);
        value = value.wrapping_add(step);
    }
    if reverse {
        data.reverse();
    }
    data
}

#[test]
fn dm1_test_vanilla() {
    map_load_save_load_save_eq("tests/dm1.map", Version::Teeworlds07);
}

#[test]
fn editor_test() {
    map_load_save_load_save_eq("tests/editor.map", Version::DDNet06);
}

#[test]
fn tile_flags_teeworlds_test() {
    let mut original_map = map_from_path("tests/tileflags_teeworlds.map");
    assert_eq!(original_map.version, Version::Teeworlds07);
    original_map.load().unwrap();
    let mut own_map = original_map.clone();

    own_map.process_tile_flag_opaque();
    own_map.set_external_image_dimensions();
    assert_eq!(own_map, original_map);
    own_map.version = Version::DDNet06;
    own_map.process_tile_flag_opaque();
    own_map.set_external_image_dimensions();
    assert_ne!(own_map, original_map);
}

#[test]
fn tile_flags_ddnet_test() {
    let mut original_map = map_from_path("tests/tileflags_ddnet.map");
    assert_eq!(original_map.version, Version::DDNet06);
    original_map.load().unwrap();
    let mut own_map = original_map.clone();

    own_map.process_tile_flag_opaque();
    own_map.set_external_image_dimensions();
    assert_eq!(own_map, original_map);
    own_map.version = Version::Teeworlds07;
    own_map.process_tile_flag_opaque();
    own_map.set_external_image_dimensions();
    assert_ne!(own_map, original_map);
}

fn p<T: Copy>(x: T, y: T) -> Vec2<T> {
    Vec2::new(x, y)
}

fn p_bits<T: Fixed, U: From<Vec2<T>>>(x: T::Bits, y: T::Bits) -> U {
    Vec2::new(T::from_bits(x), T::from_bits(y)).into()
}

fn p_num<T: Fixed>(x: T::Bits, y: T::Bits) -> Vec2<T>
where
    T::Bits: ToFixed,
{
    Vec2::new(T::from_num(x), T::from_num(y))
}

#[test]
fn custom_test_ddnet() {
    let tf: Vec<TileFlags> = (0..16).map(|n| TileFlags::from_bits(n).unwrap()).collect();

    let mut map = TwMap {
        version: Version::DDNet06,
        info: Info {
            author: String::from("Mapper with name of any length"),
            version: String::new(),
            credits: String::from(""),
            license: String::new(),
            settings: vec![
                String::from("Setting 1"),
                String::from("Unnecessary long setting abcdefghijklmnopqrstuvwxyz"),
                String::from("Setting 3"),
            ],
        },
        images: vec![
            Image::Embedded(EmbeddedImage {
                name: "name_with_128_bytes_length_restriction_image_1".to_string(),
                image: RgbaImage::from_raw(16, 32, generate_data(32 * 16 * 4, 1, false))
                    .unwrap()
                    .into(),
            }),
            Image::External(ExternalImage {
                size: Extent2::new(1337, 42),
                name: "sun".to_string(),
            }),
            Image::Embedded(EmbeddedImage {
                name: "".to_string(),
                image: RgbaImage::from_raw(64, 32, generate_data(64 * 32 * 4, 2, true))
                    .unwrap()
                    .into(),
            }),
        ],
        envelopes: vec![
            Envelope::Sound(Env {
                points: vec![
                    EnvPoint {
                        time: 0,
                        curve: CurveKind::Step,
                        content: Volume(I22F10::from_num(1)),
                    },
                    EnvPoint {
                        time: 1,
                        curve: CurveKind::Fast,
                        content: Volume(I22F10::from_num(2)),
                    },
                ],
                name: "abcdefghijklmnopqrstuvwxyzabcde".to_string(),
                synchronized: false,
            }),
            Envelope::Sound(Env {
                points: vec![
                    EnvPoint {
                        time: 0,
                        curve: CurveKind::Linear,
                        content: Volume(I22F10::from_num(0)),
                    },
                    EnvPoint {
                        time: i32::MAX,
                        curve: CurveKind::Slow,
                        content: Volume(I22F10::MAX),
                    },
                ],
                name: "".to_string(),
                synchronized: true,
            }),
            Envelope::Position(Env {
                points: vec![
                    EnvPoint {
                        time: 0,
                        curve: CurveKind::Step,
                        content: Position {
                            offset: p_num(1, 2),
                            rotation: I22F10::from_num(0),
                        },
                    },
                    EnvPoint {
                        time: i32::MAX,
                        curve: CurveKind::Fast,
                        content: Position {
                            offset: Vec2::new(I17F15::MAX, I17F15::MIN),
                            rotation: I22F10::MAX,
                        },
                    },
                ],
                name: "abcdefghijklmnopqrstuvwxyzabcde".to_string(),
                synchronized: false,
            }),
            Envelope::Color(Env {
                points: vec![
                    EnvPoint {
                        time: 0,
                        curve: CurveKind::Smooth,
                        content: Rgba {
                            r: I22F10::from_num(1),
                            g: I22F10::from_num(2),
                            b: I22F10::from_num(3),
                            a: I22F10::from_num(4),
                        },
                    },
                    EnvPoint {
                        time: i32::MAX,
                        curve: CurveKind::Linear,
                        content: Rgba {
                            r: I22F10::MAX,
                            g: I22F10::MIN,
                            b: I22F10::MAX,
                            a: I22F10::MIN,
                        },
                    },
                ],
                name: "abcdefghijklmnopqrstuvwxyzabcde".to_string(),
                synchronized: false,
            }),
        ],
        groups: vec![
            Group {
                layers: vec![
                    Layer::Game(GameLayer {
                        tiles: CompressedData::Loaded(arr2(&[
                            [
                                GameTile::new(1, tf[0]),
                                GameTile::new(2, tf[1]),
                                GameTile::new(3, tf[2]),
                            ],
                            [
                                GameTile::new(4, tf[3]),
                                GameTile::new(5, tf[8]),
                                GameTile::new(6, tf[9]),
                            ],
                        ])),
                    }),
                    Layer::Tiles(TilesLayer {
                        detail: true,
                        color: Rgba {
                            r: 1,
                            g: 2,
                            b: 3,
                            a: 4,
                        },
                        color_env: Some(3),
                        color_env_offset: 21,
                        image: Some(2),
                        tiles: CompressedData::Loaded(arr2(&[
                            [
                                Tile::new(1, tf[6]),
                                Tile::new(2, tf[7]),
                                Tile::new(3, tf[8]),
                            ],
                            [
                                Tile::new(4, tf[9]),
                                Tile::new(5, tf[10]),
                                Tile::new(6, tf[11]),
                            ],
                        ])),
                        name: "abcdefghijk".to_string(),
                        automapper_config: AutomapperConfig {
                            config: None,
                            seed: 21,
                            automatic: false,
                        },
                    }),
                ],
                ..Group::physics()
            },
            Group {
                offset: p_num(1, 2),
                parallax: p(3, 4),
                layers: vec![
                    Layer::Sounds(SoundsLayer {
                        detail: false,
                        sources: vec![
                            SoundSource {
                                area: SoundArea::Circle(Disk::new(
                                    p_bits(21, 42),
                                    I27F5::from_bits(37),
                                )),
                                looping: true,
                                panning: false,
                                delay: 21,
                                falloff: 123,
                                position_env: Some(2),
                                position_env_offset: 3,
                                sound_env: Some(0),
                                sound_env_offset: 7,
                            },
                            SoundSource {
                                area: SoundArea::Rectangle(Rect {
                                    x: I17F15::from_bits(43),
                                    y: I17F15::from_bits(68),
                                    w: I17F15::from_bits(12),
                                    h: I17F15::from_bits(34),
                                }),
                                looping: false,
                                panning: true,
                                delay: 42,
                                falloff: 246,
                                position_env: None,
                                position_env_offset: 0,
                                sound_env: Some(1),
                                sound_env_offset: 14,
                            },
                        ],
                        sound: None,
                        name: "".to_string(),
                    }),
                    Layer::Quads(QuadsLayer {
                        detail: true,
                        quads: vec![Quad {
                            position: p_bits(12, 24),
                            corners: [p_bits(1, 2), p_bits(3, 4), p_bits(5, 6), p_bits(7, 8)],
                            colors: [
                                Rgba {
                                    r: 9,
                                    g: 10,
                                    b: 11,
                                    a: 12,
                                },
                                Rgba {
                                    r: 13,
                                    g: 14,
                                    b: 15,
                                    a: 16,
                                },
                                Rgba {
                                    r: 17,
                                    g: 18,
                                    b: 19,
                                    a: 20,
                                },
                                Rgba {
                                    r: 21,
                                    g: 22,
                                    b: 23,
                                    a: 24,
                                },
                            ],
                            texture_coords: [
                                p_bits(27, 28),
                                p_bits(25, 26),
                                p_bits(29, 30),
                                p_bits(31, 32),
                            ],
                            position_env: Some(2),
                            position_env_offset: 33,
                            color_env: Some(3),
                            color_env_offset: 34,
                        }],
                        image: Some(1),
                        name: "another3i32".to_string(),
                    }),
                    Layer::Tiles(TilesLayer {
                        detail: true,
                        color: Rgba {
                            r: 21,
                            g: 42,
                            b: 63,
                            a: 84,
                        },
                        color_env: Some(3),
                        color_env_offset: 13,
                        image: Some(0),
                        tiles: CompressedData::Loaded(arr2(&[
                            [
                                Tile::new(11, tf[12]),
                                Tile::new(13, tf[13]),
                                Tile::new(15, tf[14]),
                            ],
                            [
                                Tile::new(17, tf[15]),
                                Tile::new(19, tf[14]),
                                Tile::new(21, tf[13]),
                            ],
                        ])),
                        name: "ABCDEFGHIJK".to_string(),
                        automapper_config: AutomapperConfig {
                            config: Some(1),
                            seed: 2121,
                            automatic: true,
                        },
                    }),
                ],
                clipping: true,
                clip: Rect {
                    x: I27F5::from_bits(5),
                    y: I27F5::from_bits(6),
                    w: I27F5::from_bits(7),
                    h: I27F5::from_bits(8),
                },
                name: "3intsstring".to_string(),
            },
        ],
        sounds: vec![],
    };
    map_save_load_save_eq(&mut map);
    map.version = Version::Teeworlds07;
    let mut data = Vec::new();
    assert!(map.save(&mut data).is_err());
}

#[test]
fn custom_test_teeworlds07() {
    let tf: Vec<TileFlags> = (0..16).map(|n| TileFlags::from_bits(n).unwrap()).collect();

    let mut map = TwMap {
        version: Version::Teeworlds07,
        info: Default::default(),
        images: vec![],
        envelopes: vec![
            Envelope::Color(Env {
                name: "Vanilla envelope".to_string(),
                synchronized: false,
                points: vec![
                    EnvPoint {
                        time: 0,
                        curve: CurveKind::Bezier(BezierCurve {
                            handle_l: Vec2::new(
                                Rgba {
                                    r: I22F10::from_bits(0),
                                    g: I22F10::from_bits(1),
                                    b: I22F10::from_bits(2),
                                    a: I22F10::from_bits(3),
                                },
                                Rgba {
                                    r: I22F10::from_bits(4),
                                    g: I22F10::from_bits(5),
                                    b: I22F10::from_bits(6),
                                    a: I22F10::from_bits(7),
                                },
                            ),
                            handle_r: Vec2::new(
                                Rgba {
                                    r: I22F10::from_bits(8),
                                    g: I22F10::from_bits(9),
                                    b: I22F10::from_bits(10),
                                    a: I22F10::from_bits(11),
                                },
                                Rgba {
                                    r: I22F10::from_bits(12),
                                    g: I22F10::from_bits(13),
                                    b: I22F10::from_bits(14),
                                    a: I22F10::from_bits(15),
                                },
                            ),
                        }),
                        content: Rgba {
                            r: I22F10::from_bits(0),
                            g: I22F10::from_bits(7),
                            b: I22F10::from_bits(14),
                            a: I22F10::from_bits(21),
                        },
                    },
                    EnvPoint {
                        time: 0,
                        curve: CurveKind::Linear,
                        content: Rgba {
                            r: I22F10::from_bits(28),
                            g: I22F10::from_bits(35),
                            b: I22F10::from_bits(42),
                            a: I22F10::from_bits(49),
                        },
                    },
                ],
            }),
            Envelope::Position(Env {
                name: "".to_string(),
                synchronized: true,
                points: vec![
                    EnvPoint {
                        time: 1,
                        content: Position {
                            offset: p_bits(123, 456),
                            rotation: I22F10::from_bits(789),
                        },
                        curve: CurveKind::Bezier(BezierCurve {
                            handle_l: Vec2::new(
                                Position {
                                    offset: p_bits(16, 17),
                                    rotation: I22F10::from_bits(18),
                                },
                                Position {
                                    offset: p_bits(19, 20),
                                    rotation: I22F10::from_bits(21),
                                },
                            ),
                            handle_r: Vec2::new(
                                Position {
                                    offset: p_bits(22, 23),
                                    rotation: I22F10::from_bits(24),
                                },
                                Position {
                                    offset: p_bits(25, 26),
                                    rotation: I22F10::from_bits(27),
                                },
                            ),
                        }),
                    },
                    EnvPoint {
                        time: 1,
                        content: Position {
                            offset: p_bits(987, 654),
                            rotation: I22F10::from_bits(321),
                        },
                        curve: CurveKind::Bezier(BezierCurve {
                            handle_l: Vec2::new(
                                Position {
                                    offset: p_bits(101, 102),
                                    rotation: I22F10::from_bits(103),
                                },
                                Position {
                                    offset: p_bits(104, 105),
                                    rotation: I22F10::from_bits(106),
                                },
                            ),
                            handle_r: Vec2::new(
                                Position {
                                    offset: p_bits(0, 0),
                                    rotation: I22F10::from_bits(0),
                                },
                                Position {
                                    offset: p_bits(0, 0),
                                    rotation: I22F10::from_bits(0),
                                },
                            ),
                        }),
                    },
                ],
            }),
        ],
        groups: vec![Group {
            layers: vec![Layer::Game(GameLayer {
                tiles: CompressedData::Loaded(arr2(&[
                    [
                        GameTile::new(1, tf[0]),
                        GameTile::new(2, tf[1]),
                        GameTile::new(3, tf[2]),
                    ],
                    [
                        GameTile::new(4, tf[3]),
                        GameTile::new(5, tf[8]),
                        GameTile::new(6, tf[9]),
                    ],
                ])),
            })],
            ..Group::physics()
        }],
        sounds: vec![],
    };

    map_save_load_save_eq(&mut map);
    map.version = Version::DDNet06;
    map_save_load_save_eq(&mut map);
}

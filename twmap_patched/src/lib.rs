//! This crate provides a library for safe parsing, editing and saving of [Teeworlds](https://www.teeworlds.com/) and [DDNet](https://ddnet.org/) maps.
//! Goals of this library are:
//!
//! - performance
//! - reasonable map standards
//! - compatibility with all Teeworlds [maps](https://github.com/teeworlds/teeworlds-maps) (0.7) and DDNet (0.6) [maps](https://github.com/ddnet/ddnet-maps)
//!
//! In the very center of this library is the [`TwMap`](struct.TwMap.html) struct.
//! For more information and the complete list of its available methods, check out its documentation page.
//!
//! Note that library is a aware of the origin of the map (Teeworlds 0.7 or DDNet 0.6).
//! On parsing a map it will store its [`Version`](enum.Version.html) in the `version` field, which in turn is used to perform the appropriate checks and save in the correct version.
//!
//!
//! # Examples:
//!
//! Note that for better readability, some lines of code are left out in the examples, most notably `use twmap::*;` at the top.
//!
//! Loading a binary map, accessing its version and image names and saving it again in the binary format:
//! ```
//! # use std::path::PathBuf;
//! # use twmap::*;
//! # use std::fs;
//! #
//! # let dm1_path = PathBuf::from("tests/dm1.map");
//! let map_data: Vec<u8> = fs::read(dm1_path)?;
//! let mut map: TwMap = TwMap::parse(&map_data)?;
//!
//! assert_eq!(map.version, Version::Teeworlds07);
//! assert_eq!(map.images[0].name(), "bg_cloud1");
//! assert_eq!(map.images[6].name(), "sun");
//!
//! # let mut tmp_dir = tempfile::tempdir().unwrap();
//! # let output_path = tmp_dir.path().join("dm1_new").with_extension("map");
//! map.save_file(output_path)?;
//! # Ok::<(), Error>(())
//! ```
//!
//! Loading a MapDir **or** binary map, removing unused envelopes, layers, groups, images, sounds and saving it again in the MapDir format:
//! ```
//! # use std::path::PathBuf;
//! # use twmap::*;
//! #
//! # let dm1_path = PathBuf::from("tests/dm1.map");
//! # let mut dm1 = TwMap::parse_path(dm1_path)?;
//! # let tmp_dir = tempfile::tempdir().unwrap();
//! # let map_path = tmp_dir.path().join("map").with_extension("map");
//! # dm1.save_dir(&map_path);
//! let mut map = TwMap::parse_path(map_path)?;
//!
//! map.remove_everything_unused();
//!
//! # let tmp_dir = tempfile::tempdir().unwrap();
//! # let output_path = tmp_dir.path().join("map_new");
//! map.save_dir(output_path)?;
//! # Ok::<(), Error>(())
//! ```
//!
//! Loading a MapDir map, flipping it upside down and saving the map data into a vector:
//!
//! ```
//! # use std::path::PathBuf;
//! # use twmap::*;
//! #
//! # let dm1_path = PathBuf::from("tests/dm1.map");
//! # let mut dm1 = TwMap::parse_path(dm1_path)?;
//! # let tmp_dir = tempfile::tempdir().unwrap();
//! # let map_path = tmp_dir.path().join("map").with_extension("map");
//! # dm1.save_dir(&map_path);
//! let mut map = TwMap::parse_dir(map_path)?;
//! # // TODO: Fix this as soon as NoneError is stable
//! // Usually you would match the Option<TwMap> return values
//! // -> or wrap it into a function that also returns an Option
//! // To take up less space, we insert an error that we return in place of the None value
//! # let e = std::io::Error::from_raw_os_error(21);
//! map = map.rotate_right().ok_or(e)?;
//! # let e = std::io::Error::from_raw_os_error(21);
//! map = map.rotate_right().ok_or(e)?;
//! # let e = std::io::Error::from_raw_os_error(21);
//! map = map.mirror().ok_or(e)?;
//!
//! let mut map_data = Vec::new();
//! map.save(&mut map_data)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod datafile;
mod map;
pub use map::*;

pub mod automapper;
pub mod constants;

pub mod compression; // safe zlib (libz-sys) wrapper
pub mod convert;

pub use fixed;
pub use image;
pub use ndarray;
pub use vek;

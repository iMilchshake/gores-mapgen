//! # Gores Map Generator
//!
//! Procedural random map generator for DDNet maps of the gores type ([see DDNet wiki](https://wiki.ddnet.org/wiki/Gores)) using a random walker based algorithm
//! with various post-processing steps.
//!
//! ## ⚠️ Work in Progress
//!
//! This project is actively under development. Both the codebase and documentation are subject
//! to significant changes. The documentation may lag behind code changes, contain incomplete
//! information, or be outdated in places. Pull requests helping improve the documentation are
//! very welcome!
//!
//! # Main Components
//!
//! - [`generator`]: Orchestrates the entire generation process
//! - [`walker`]: Carves playable path using random walker
//! - [`post_processing`]: Refines the generated map with various post processing steps
//! - [`config`]: Configuration structs for generation, map layout, and theme settings
//! - [`map`]: Core map data structure and block types
//! - [`kernel`]: Kernel shapes used by the walker for carving paths

pub mod args;
pub mod config;
pub mod debug;
pub mod dt;
pub mod editor;
pub mod generator;
pub mod gui;
pub mod kernel;
pub mod map;
pub mod map_camera;
pub mod noise;
pub mod position;
pub mod post_processing;
pub mod random;
pub mod rendering;
pub mod twmap_export;
pub mod utils;
pub mod walker;

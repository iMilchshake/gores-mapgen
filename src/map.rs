use crate::{position::Position, walker::CuteWalker};
use ndarray::{s, Array2};
use std::path::PathBuf;
use twmap::{GameLayer, GameTile, TileFlags, TilemapLayer, TwMap};

const CHUNK_SIZE: usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Empty,
    Hookable,
    Freeze,
    Spawn,
    Start,
    Finish,
    Platform,
}

impl BlockType {
    /// maps BlockType to tw game layer id for map export
    fn to_tw_game_id(&self) -> u8 {
        match self {
            BlockType::Empty => 0,
            BlockType::Hookable => 1,
            BlockType::Platform => 1,
            BlockType::Freeze => 9,
            BlockType::Spawn => 192,
            BlockType::Start => 33,
            BlockType::Finish => 34,
        }
    }
}

pub enum KernelType {
    Outer,
    Inner,
}

#[derive(Debug)]
pub struct Map {
    pub grid: Array2<BlockType>,
    pub height: usize,
    pub width: usize,
    pub spawn: Position,

    pub chunk_edited: Array2<bool>, // TODO: make this optional in case editor is not used!
    pub chunk_size: usize,
}

fn get_maps_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        dirs::data_dir().unwrap().join("Teeworlds").join("maps")
    } else if cfg!(target_os = "linux") {
        dirs::home_dir()
            .unwrap()
            .join(".local")
            .join("share")
            .join("ddnet")
            .join("maps")
    } else {
        panic!("Unsupported operating system");
    }
}

impl Map {
    pub fn new(width: usize, height: usize, default: BlockType, spawn: Position) -> Map {
        Map {
            grid: Array2::from_elem((width, height), default),
            width,
            height,
            spawn,
            chunk_edited: Array2::from_elem(
                (width.div_ceil(CHUNK_SIZE), height.div_ceil(CHUNK_SIZE)),
                false,
            ),
            chunk_size: CHUNK_SIZE,
        }
    }

    pub fn update(
        &mut self,
        walker: &CuteWalker,
        kernel_type: KernelType,
    ) -> Result<(), &'static str> {
        let kernel = match kernel_type {
            KernelType::Inner => &walker.inner_kernel,
            KernelType::Outer => &walker.outer_kernel,
        };
        let offset: usize = kernel.size / 2; // offset of kernel wrt. position (top/left)
        let extend: usize = kernel.size - offset; // how much kernel extends position (bot/right)

        let exceeds_left_bound = walker.pos.x < offset;
        let exceeds_upper_bound = walker.pos.y < offset;
        let exceeds_right_bound = (walker.pos.x + extend) > self.width;
        let exceeds_lower_bound = (walker.pos.y + extend) > self.height;

        if exceeds_left_bound || exceeds_upper_bound || exceeds_right_bound || exceeds_lower_bound {
            return Err("kernel out of bounds");
        }

        let root_pos = Position::new(walker.pos.x - offset, walker.pos.y - offset);
        for ((kernel_x, kernel_y), kernel_active) in kernel.vector.indexed_iter() {
            let absolute_pos = Position::new(root_pos.x + kernel_x, root_pos.y + kernel_y);
            if *kernel_active {
                let current_type = &self.grid[absolute_pos.as_index()];

                let new_type = match (&kernel_type, current_type) {
                    // inner kernel removes everything
                    (KernelType::Inner, BlockType::Hookable) => Some(BlockType::Empty),
                    (KernelType::Inner, BlockType::Freeze) => Some(BlockType::Empty),

                    // outer kernel will turn hookables to freeze
                    (KernelType::Outer, BlockType::Hookable) => Some(BlockType::Freeze),
                    (KernelType::Outer, BlockType::Freeze) => Some(BlockType::Freeze),

                    // ignore everything else
                    (_, _) => None,
                };

                if let Some(new_type) = new_type {
                    self.grid[absolute_pos.as_index()] = new_type;
                }

                let chunk_pos = self.pos_to_chunk_pos(absolute_pos);
                self.chunk_edited[chunk_pos.as_index()] = true;
            }
        }

        Ok(())
    }

    pub fn generate_room(&mut self, pos: &Position, margin: usize, zone_type: Option<&BlockType>) {
        // TODO: ensure valid position?
        // TODO: use new shift method here

        // carve room
        self.set_area(
            &Position::new(pos.x - margin, pos.y - margin),
            &Position::new(pos.x + margin, pos.y + margin),
            &BlockType::Empty,
            true,
        );

        // set platform
        self.set_area(
            &Position::new(pos.x - (margin - 2), pos.y),
            &Position::new(pos.x + (margin - 2), pos.y),
            &BlockType::Platform,
            true,
        );

        // set spawns
        if zone_type == Some(&BlockType::Start) {
            self.set_area(
                &Position::new(pos.x - (margin - 2), pos.y - 1),
                &Position::new(pos.x + (margin - 2), pos.y - 1),
                &BlockType::Spawn,
                true,
            );
        }

        // set start/finish line
        if let Some(zone_type) = zone_type {
            self.set_area_border(
                &Position::new(pos.x - margin - 1, pos.y - margin - 1),
                &Position::new(pos.x + margin + 1, pos.y + margin + 1),
                zone_type,
                false,
            );
        }
    }

    fn pos_to_chunk_pos(&self, pos: Position) -> Position {
        Position::new(pos.x / CHUNK_SIZE, pos.y / CHUNK_SIZE)
    }

    pub fn export(&self, path: &PathBuf) {
        let mut map = TwMap::parse_file("test.map").expect("parsing failed");
        map.load().expect("loading failed");

        // get game layer
        let game_layer = map
            .find_physics_layer_mut::<GameLayer>()
            .unwrap()
            .tiles_mut()
            .unwrap_mut();

        *game_layer = Array2::<GameTile>::from_elem(
            (self.width, self.height),
            GameTile::new(0, TileFlags::empty()),
        );

        // modify game layer
        for ((x, y), value) in self.grid.indexed_iter() {
            game_layer[[y, x]] = GameTile::new(value.to_tw_game_id(), TileFlags::empty())
        }

        // save map
        println!("exporting map to {:?}", &path);
        map.save_file(path).expect("saving failed");
    }

    pub fn pos_in_bounds(&self, pos: &Position) -> bool {
        // we dont have to check for lower bound, because of usize
        pos.x < self.width && pos.y < self.height
    }

    pub fn check_area_exists(
        &self,
        top_left: &Position,
        bot_right: &Position,
        value: &BlockType,
    ) -> bool {
        let area = self.grid.slice(s![
            top_left.x..=bot_right.x + 1,
            top_left.y..=bot_right.y + 1
        ]);

        area.iter().any(|block| block == value)
    }

    pub fn check_area_all(
        &self,
        top_left: &Position,
        bot_right: &Position,
        value: &BlockType,
    ) -> bool {
        let area = self.grid.slice(s![
            top_left.x..=bot_right.x + 1,
            top_left.y..=bot_right.y + 1
        ]);

        area.iter().all(|block| block == value)
    }

    // TODO: right now override is hardcoded to overide empty AND freeze. i might need some
    // distiction here in the future :)
    pub fn set_area(
        &mut self,
        top_left: &Position,
        bot_right: &Position,
        value: &BlockType,
        overide: bool,
    ) {
        if !self.pos_in_bounds(top_left) || !self.pos_in_bounds(bot_right) {
            return;
        }

        let mut view = self
            .grid
            .slice_mut(s![top_left.x..bot_right.x + 1, top_left.y..bot_right.y + 1]);
        view.map_inplace(|current_value| {
            if overide || *current_value == BlockType::Empty || *current_value == BlockType::Freeze
            {
                *current_value = value.clone();
            }
        });
    }

    /// sets the outline of an area define by two positions
    pub fn set_area_border(
        &mut self,
        top_left: &Position,
        bot_right: &Position,
        value: &BlockType,
        overide: bool,
    ) {
        let top_right = Position::new(bot_right.x, top_left.y);
        let bot_left = Position::new(top_left.x, bot_right.y);

        self.set_area(&top_left, &top_right, value, overide);
        self.set_area(&top_right, &bot_right, value, overide);
        self.set_area(&top_left, &bot_left, value, overide);
        self.set_area(&bot_left, &bot_right, value, overide);
    }
}

use crate::{position::Position, walker::CuteWalker};
use ndarray::Array2;
use twmap::{GameLayer, GameTile, TileFlags, TilemapLayer, TwMap};

const CHUNK_SIZE: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockType {
    Empty,
    Hookable,
    Freeze,
}

pub enum KernelType {
    Outer,
    Inner,
}

#[derive(Debug, Clone)]
pub struct Map {
    pub grid: Array2<BlockType>,
    pub height: usize,
    pub width: usize,
    pub spawn: Position,

    pub chunk_edited: Array2<bool>, // TODO: make this optional in case editor is not used!
    pub chunk_size: usize,
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
                let current_type = self.grid[absolute_pos.as_index()];
                let new_type = match (&kernel_type, current_type) {
                    // inner kernel removes everything
                    (KernelType::Inner, _) => BlockType::Empty,

                    // outer kernel will turn hookables to freeze
                    (KernelType::Outer, BlockType::Hookable) => BlockType::Freeze,
                    (KernelType::Outer, BlockType::Freeze) => BlockType::Freeze,
                    (KernelType::Outer, BlockType::Empty) => BlockType::Empty,
                };
                self.grid[absolute_pos.as_index()] = new_type;

                let chunk_pos = self.pos_to_chunk_pos(absolute_pos);
                self.chunk_edited[chunk_pos.as_index()] = true;
            }
        }

        Ok(())
    }

    fn pos_to_chunk_pos(&self, pos: Position) -> Position {
        Position::new(pos.x / CHUNK_SIZE, pos.y / CHUNK_SIZE)
    }

    pub fn export(&self, name: String) {
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
            game_layer[[y, x]] = match value {
                BlockType::Empty => GameTile::new(0, TileFlags::empty()),
                BlockType::Hookable => GameTile::new(1, TileFlags::empty()),
                BlockType::Freeze => GameTile::new(9, TileFlags::empty()),
            };
        }

        game_layer[self.spawn.as_index()] = GameTile::new(192, TileFlags::empty());

        // save map
        map.save_file(name + ".map").expect("saving failed");
    }
}

use crate::{Map, Position, ShiftDirection};

// this walker is indeed very cute
#[derive(Debug)]
pub struct CuteWalker {
    pub pos: Position,
}

impl CuteWalker {
    pub fn new(initial_pos: Position) -> CuteWalker {
        CuteWalker { pos: initial_pos }
    }

    pub fn cuddle(&self) {
        println!("Cute walker was cuddled!");
    }

    pub fn shift_pos(&mut self, shift: ShiftDirection, map: &Map) -> Result<(), &str> {
        if !self.is_shift_valid(&shift, map) {
            return Err("invalid shift");
        }

        match shift {
            ShiftDirection::Up => self.pos.y -= 1,
            ShiftDirection::Right => self.pos.x += 1,
            ShiftDirection::Down => self.pos.y += 1,
            ShiftDirection::Left => self.pos.x -= 1,
        }

        Ok(())
    }

    pub fn is_shift_valid(&self, shift: &ShiftDirection, map: &Map) -> bool {
        match shift {
            ShiftDirection::Up => self.pos.y > 0,
            ShiftDirection::Right => self.pos.x < map.width - 1,
            ShiftDirection::Down => self.pos.y < map.height - 1,
            ShiftDirection::Left => self.pos.x > 0,
        }
    }
}

use crate::*;

impl TileFlags {
    /// Mirrors the tile, switches left <-> right.
    /// **Doesn't work with directional physics tiles!**
    ///
    /// Use [TileFlips::flip_x] instead!
    pub fn flip_x(&mut self) {
        match self.contains(TileFlags::ROTATE) {
            false => self.toggle(TileFlags::FLIP_X),
            true => self.toggle(TileFlags::FLIP_Y),
        }
    }

    /// Mirrors the tile vertically, switches up <-> down.
    /// **Doesn't work with directional physics tiles!**
    ///
    /// Use [TileFlips::flip_y] instead!
    pub fn flip_y(&mut self) {
        match self.contains(TileFlags::ROTATE) {
            false => self.toggle(TileFlags::FLIP_Y),
            true => self.toggle(TileFlags::FLIP_X),
        }
    }

    /// Performs a 90° clockwise rotation.
    ///
    /// For generic programming, use [TileFlips::rotate_cw] instead!
    pub fn rotate_cw(&mut self) {
        if self.contains(TileFlags::ROTATE) {
            self.toggle(TileFlags::FLIP_Y | TileFlags::FLIP_X);
        }
        self.toggle(TileFlags::ROTATE);
    }

    /// Performs a 90° counterclockwise rotation.
    ///
    /// For generic programming, use [TileFlips::rotate_ccw] instead!
    pub fn rotate_ccw(&mut self) {
        if !self.contains(TileFlags::ROTATE) {
            self.toggle(TileFlags::FLIP_Y | TileFlags::FLIP_X);
        }
        self.toggle(TileFlags::ROTATE);
    }

    /// In contrast to the [`TilesLayer`], the directional tiles of physics layers only have 4 valid rotation.
    /// The rotate methods keep the correct state, `flip_h` and `flip_v` on the other hand don't.
    pub fn flip_x_physics(&mut self) {
        if self.contains(TileFlags::ROTATE) {
            self.toggle(TileFlags::FLIP_Y | TileFlags::FLIP_X);
        }
    }

    /// In contrast to the [`TilesLayer`], the directional tiles of physics layers only have 4 valid rotation.
    /// The rotate methods keep the correct state, `flip_h` and `flip_v` on the other hand don't.
    pub fn flip_y_physics(&mut self) {
        if !self.contains(TileFlags::ROTATE) {
            self.toggle(TileFlags::FLIP_Y | TileFlags::FLIP_X);
        }
    }
}

trait TilePrivate {
    fn mirror_id(id: u8) -> u8 {
        id
    }
    fn directional_physics_tile(_id: u8) -> bool {
        false
    }
}

impl TilePrivate for Tile {}

fn mirror_lasers(mut id: u8) -> u8 {
    if (203..=205).contains(&id) {
        id += (206 - id) * 2;
    } else if (207..=209).contains(&id) {
        id -= (id - 206) * 2;
    }
    id
}

impl TilePrivate for GameTile {
    fn mirror_id(id: u8) -> u8 {
        mirror_lasers(id)
    }

    fn directional_physics_tile(id: u8) -> bool {
        matches!(id, 60 | 61 | 64 | 65 | 67 | 224 | 225)
    }
}
impl TilePrivate for Tele {}
impl TilePrivate for Switch {
    fn mirror_id(id: u8) -> u8 {
        mirror_lasers(id)
    }
    fn directional_physics_tile(id: u8) -> bool {
        matches!(id, 224 | 225)
    }
}
impl TilePrivate for Tune {}

/// Generic tile flips and rotates for every tile type.
pub trait TileFlips: AnyTile {
    /// Left <-> Right
    fn flip_x(&mut self);
    /// Up <-> Down
    fn flip_y(&mut self);
    /// Rotates clockwise
    fn rotate_cw(&mut self);
    /// Rotates counterclockwise
    fn rotate_ccw(&mut self);
}

impl<T: AnyTile + TilePrivate> TileFlips for T {
    fn flip_x(&mut self) {
        let id = self.id();
        if let Some(flags) = self.flags_mut() {
            if T::directional_physics_tile(id) {
                flags.flip_x_physics();
            } else {
                flags.flip_x();
            }
            *self.id_mut() = T::mirror_id(*self.id_mut());
        }
    }

    fn flip_y(&mut self) {
        let id = self.id();
        if let Some(flags) = self.flags_mut() {
            if T::directional_physics_tile(id) {
                flags.flip_y_physics();
            } else {
                flags.flip_y();
            }
            *self.id_mut() = T::mirror_id(*self.id_mut());
        }
    }

    fn rotate_cw(&mut self) {
        if let Some(flags) = self.flags_mut() {
            flags.rotate_cw();
        }
    }

    fn rotate_ccw(&mut self) {
        if let Some(flags) = self.flags_mut() {
            flags.rotate_ccw();
        }
    }
}

impl TileFlips for Speedup {
    fn flip_x(&mut self) {
        let mut angle = i16::from(self.angle);
        angle = 180 - angle;
        if angle < 0 {
            angle += 360;
        }
        self.angle = angle.into();
    }

    fn flip_y(&mut self) {
        let mut angle = i16::from(self.angle);
        angle = 180 - angle;
        angle *= -1;
        angle = 180 - angle;
        angle %= 360;
        self.angle = angle.into();
    }

    fn rotate_cw(&mut self) {
        let mut angle = i16::from(self.angle);
        angle += 90;
        angle %= 360;
        self.angle = angle.into();
    }

    fn rotate_ccw(&mut self) {
        let mut angle = i16::from(self.angle);
        angle += 270;
        angle %= 360;
        self.angle = angle.into();
    }
}

pub trait EditTile {
    fn tile(_tile: &mut Tile) {}
    fn game_tile(_tile: &mut GameTile) {}
    fn tele(_tele: &mut Tele) {}
    fn speedup(_speedup: &mut Speedup) {}
    fn switch(_switch: &mut Switch) {}
    fn tune(_tune: &mut Tune) {}
}

fn edit_tilemap<T: TilemapLayer, F: Fn(&mut T::TileType)>(layer: &mut T, f: F) {
    layer.tiles_mut().unwrap_mut().iter_mut().for_each(f)
}

impl TwMap {
    /// Requires the tiles to be loaded
    pub fn edit_tiles<T: EditTile>(&mut self) {
        for group in &mut self.groups {
            for layer in &mut group.layers {
                match layer {
                    Layer::Game(l) => edit_tilemap(l, T::game_tile),
                    Layer::Tiles(l) => edit_tilemap(l, T::tile),
                    Layer::Front(l) => edit_tilemap(l, T::game_tile),
                    Layer::Tele(l) => edit_tilemap(l, T::tele),
                    Layer::Speedup(l) => edit_tilemap(l, T::speedup),
                    Layer::Switch(l) => edit_tilemap(l, T::switch),
                    Layer::Tune(l) => edit_tilemap(l, T::tune),
                    Layer::Quads(_) | Layer::Sounds(_) | Layer::Invalid(_) => {}
                }
            }
        }
    }
}

pub struct ZeroUnusedParts;

impl TileFlags {
    fn clear_unused(&mut self) -> bool {
        let cleared = *self & TileFlags::all();
        let changed = cleared != *self;
        *self = cleared;
        changed
    }

    fn clear_unused_and_opaque(&mut self) -> bool {
        let cleared = *self & (TileFlags::all() - TileFlags::OPAQUE);
        let changed = cleared != *self;
        *self = cleared;
        changed
    }
}

impl EditTile for ZeroUnusedParts {
    fn tile(tile: &mut Tile) {
        tile.flags.clear_unused();
        tile.unused = 0;
        tile.skip = 0;
    }
    fn game_tile(tile: &mut GameTile) {
        tile.flags.clear_unused_and_opaque();
        tile.unused = 0;
        tile.skip = 0;
    }
    fn speedup(speedup: &mut Speedup) {
        speedup.unused_padding = 0;
    }
    fn switch(switch: &mut Switch) {
        switch.flags.clear_unused_and_opaque();
    }
}

fn zero_air_tile<T: AnyTile>(tile: &mut T) {
    if tile.id() == 0 {
        *tile = T::default()
    }
}

/// To zero out tiles with id 0 completely
pub struct ZeroAir;

impl EditTile for ZeroAir {
    fn tile(tile: &mut Tile) {
        zero_air_tile(tile)
    }
    fn game_tile(tile: &mut GameTile) {
        zero_air_tile(tile)
    }
    fn tele(tele: &mut Tele) {
        zero_air_tile(tele)
    }
    fn speedup(speedup: &mut Speedup) {
        zero_air_tile(speedup)
    }
    fn switch(switch: &mut Switch) {
        zero_air_tile(switch)
    }
    fn tune(tune: &mut Tune) {
        zero_air_tile(tune)
    }
}

fn reset_unnecessary_rotation<T: TilePrivate + AnyTile>(tile: &mut T) {
    if !T::directional_physics_tile(tile.id()) {
        if let Some(flags) = tile.flags_mut() {
            flags.remove(TileFlags::ROTATE | TileFlags::FLIP_X | TileFlags::FLIP_Y);
        }
    }
}

/// DDNet specific!
/// Resets the rotation of physics tile that are not effected by rotations.
pub struct DDNetNormalizePhysicsRotation;

// We ignore Tiles layers as non-physics tiles.
// We ignore Speedup tiles as they do not have tile flags.
impl EditTile for DDNetNormalizePhysicsRotation {
    fn game_tile(tile: &mut GameTile) {
        reset_unnecessary_rotation(tile);
    }
    fn tele(tele: &mut Tele) {
        reset_unnecessary_rotation(tele);
    }
    fn switch(switch: &mut Switch) {
        reset_unnecessary_rotation(switch);
    }
    fn tune(tune: &mut Tune) {
        reset_unnecessary_rotation(tune);
    }
}

fn fix_broken_physics_rotation<T: TilePrivate + AnyTile>(tile: &mut T) {
    if T::directional_physics_tile(tile.id()) {
        let Some(flags) = tile.flags_mut() else {
            return;
        };
        if flags.contains(TileFlags::FLIP_X) ^ flags.contains(TileFlags::FLIP_Y) {
            if flags.contains(TileFlags::ROTATE) {
                flags.insert(TileFlags::FLIP_X | TileFlags::FLIP_Y);
            } else {
                flags.remove(TileFlags::FLIP_X | TileFlags::FLIP_Y);
            }
        }
    }
}

/// Fixes incorrect orientation of directional physics tiles.
pub struct DDNetFixPhysicsRotation;

impl EditTile for DDNetFixPhysicsRotation {
    fn game_tile(tile: &mut GameTile) {
        fix_broken_physics_rotation(tile);
    }
    fn tele(tele: &mut Tele) {
        fix_broken_physics_rotation(tele);
    }
    fn switch(switch: &mut Switch) {
        fix_broken_physics_rotation(switch);
    }
    fn tune(tune: &mut Tune) {
        fix_broken_physics_rotation(tune);
    }
}

/// Migrates tile id's to a newer replacement.
/// Only updates tiles that have a drop-in replacement.
pub struct DDNetMigrateSpeedup;

impl EditTile for DDNetMigrateSpeedup {
    fn speedup(speedup: &mut Speedup) {
        if speedup.id == 28 {
            speedup.id = 29;
            if (1..5).contains(&speedup.max_speed) {
                speedup.max_speed = 5;
            }
        }
    }
}

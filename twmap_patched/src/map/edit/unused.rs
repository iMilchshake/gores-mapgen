use crate::convert::To;
use crate::*;

use az::UnwrappedAs;
use ndarray::Array2;

impl TwMap {
    /// Removes duplicate physics layers of each type.
    /// Note that every map must have at most 1 physics layer of each type to pass the checks.
    /// The removal process prioritizes removing the layers in front.
    pub fn remove_duplicate_physics_layers(&mut self) -> u16 {
        let mut does_exist = [false; 7];
        let mut removed = 0;

        for group in self.groups.iter_mut().rev() {
            for i in (0..group.layers.len()).rev() {
                let variant = group.layers[i].kind();
                if variant.is_physics_layer() {
                    if does_exist[variant.duplicate_index()] {
                        group.layers.remove(i);
                        removed += 1;
                    }
                    does_exist[variant.duplicate_index()] = true;
                }
            }
        }

        removed
    }
}

fn all_tiles_default<T: AnyTile>(tiles: &Array2<T>) -> bool {
    tiles.iter().all(|t| *t == T::default())
}

impl Layer {
    /// Returns `true` if the layer has no content.
    /// - Tile map layers -> all tiles zeroed
    /// - Quads layers -> no Quads
    /// - Sounds layers -> no SoundSources
    pub fn is_empty(&self) -> bool {
        use Layer::*;
        match self {
            Game(_) => false, // Game layer must not be removed
            Tiles(l) => all_tiles_default(l.tiles.unwrap_ref()),
            Quads(l) => l.quads.is_empty(),
            Front(l) => all_tiles_default(l.tiles.unwrap_ref()),
            Tele(l) => all_tiles_default(l.tiles.unwrap_ref()),
            Speedup(l) => all_tiles_default(l.tiles.unwrap_ref()),
            Switch(l) => all_tiles_default(l.tiles.unwrap_ref()),
            Tune(l) => all_tiles_default(l.tiles.unwrap_ref()),
            Sounds(l) => l.sources.is_empty(),
            Invalid(_) => false,
        }
    }
}

impl TwMap {
    /// Combination of every other `remove_unused` method.
    pub fn remove_everything_unused(&mut self) -> u32 {
        let mut removed = 0;
        removed += self.remove_unused_layers().to::<u32>();
        removed += self.remove_unused_groups().to::<u32>();
        let (removed_external, removed_embedded) = self.remove_unused_images();
        removed += removed_external.to::<u32>();
        removed += removed_embedded.to::<u32>();
        removed += self.remove_unused_envelopes().to::<u32>();
        removed += self.remove_unused_sounds().to::<u32>();
        removed
    }

    /// Removes all layers for which [`is_empty`](enum.Layer.html#method.is_empty) returns `true`.
    pub fn remove_unused_layers(&mut self) -> u16 {
        let mut removed = 0;
        for group in &mut self.groups {
            let mut i = 0;
            while i < group.layers.len() {
                if group.layers[i].is_empty() && group.layers[i].kind() != LayerKind::Game {
                    removed += 1;
                    group.layers.remove(i);
                } else {
                    i += 1;
                }
            }
        }
        removed
    }

    /// Removes all groups that contain zero layers.
    pub fn remove_unused_groups(&mut self) -> u16 {
        let mut removed = 0;
        let mut i = 0;
        while i < self.groups.len() {
            if self.groups[i].layers.is_empty() {
                removed += 1;
                self.groups.remove(i);
            } else {
                i += 1;
            }
        }
        removed
    }

    /// Returns `true` if the image index is set for a tiles layer or a quad.
    pub fn is_image_in_use(&self, index: u16) -> bool {
        for group in &self.groups {
            for layer in &group.layers {
                match layer {
                    Layer::Tiles(l) => {
                        if l.image == Some(index) {
                            return true;
                        }
                    }
                    Layer::Quads(l) => {
                        if l.image == Some(index) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        false
    }

    /// Removes all images for which [`is_image_in_use`](struct.TwMap.html#method.is_image_in_use) returns `false`.
    /// Return value: (count of removed external images, count of removed embedded images)
    pub fn remove_unused_images(&mut self) -> (u16, u16) {
        let mut removed_external = 0;
        let mut removed_embedded = 0;
        let mut i = 0;
        while i < self.images.len() {
            if self.is_image_in_use(i.unwrapped_as()) {
                i += 1;
            } else {
                let removed_image = self.images.remove(i);
                match removed_image {
                    Image::External(_) => removed_external += 1,
                    Image::Embedded(_) => removed_embedded += 1,
                }
                self.edit_image_indices(|index| {
                    index.map(|index| {
                        if index.to::<usize>() > i {
                            index - 1
                        } else {
                            index
                        }
                    })
                })
            }
        }
        (removed_external, removed_embedded)
    }

    /// Returns `true` if the sound index is set for a sounds layer.
    pub fn is_sound_in_use(&self, index: u16) -> bool {
        for group in &self.groups {
            for layer in &group.layers {
                if let Layer::Sounds(l) = layer {
                    if l.sound == Some(index) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Removes all sounds for which [`is_sound_in_use`](struct.TwMap.html#method.is_sound_in_use) returns `false`.
    pub fn remove_unused_sounds(&mut self) -> u16 {
        let mut removed = 0;
        let mut i = 0;
        while i < self.sounds.len() {
            if self.is_sound_in_use(i.unwrapped_as()) {
                i += 1;
            } else {
                removed += 1;
                self.sounds.remove(i);
                self.edit_sound_indices(|index| {
                    index.map(|index| {
                        if index.to::<usize>() > i {
                            index - 1
                        } else {
                            index
                        }
                    })
                })
            }
        }
        removed
    }

    /// Returns `true` if the envelope index is in use.
    pub fn is_env_in_use(&self, index: u16) -> bool {
        for group in &self.groups {
            for layer in &group.layers {
                use Layer::*;
                match layer {
                    Tiles(l) => {
                        if l.color_env == Some(index) {
                            return true;
                        }
                    }
                    Quads(l) => {
                        for quad in &l.quads {
                            if quad.color_env == Some(index) {
                                return true;
                            }
                            if quad.position_env == Some(index) {
                                return true;
                            }
                        }
                    }
                    Sounds(l) => {
                        for source in &l.sources {
                            if source.position_env == Some(index) {
                                return true;
                            }
                            if source.sound_env == Some(index) {
                                return true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        false
    }

    /// Removes all sounds for which [`is_env_in_use`](struct.TwMap.html#method.is_env_in_use) returns `false`.
    pub fn remove_unused_envelopes(&mut self) -> u16 {
        let mut removed = 0;
        let mut i = 0;
        while i < self.envelopes.len() {
            if self.is_env_in_use(i.unwrapped_as()) {
                i += 1;
            } else {
                removed += 1;
                self.envelopes.remove(i);
                self.edit_env_indices(|index| {
                    index.map(|index| {
                        if index.to::<usize>() > i {
                            index - 1
                        } else {
                            index
                        }
                    })
                })
            }
        }
        removed
    }
}

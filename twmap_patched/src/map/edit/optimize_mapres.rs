use fixed::types::I32F0;
use ndarray::Array2;
use vek::Vec2;

use crate::{Layer, TwMap};

use super::Image;

impl TwMap {
    pub fn optimize_mapres(&mut self) {
        // Indicates which parts of the mapres is used by a tiles layer.
        let mut tilemap_uses: Vec<[[bool; 16]; 16]> =
            self.images.iter().map(|_| [[false; 16]; 16]).collect();
        for layer in self.groups.iter().flat_map(|g| g.layers.iter()) {
            if let Layer::Tiles(l) = layer {
                let mapres = match l.image {
                    None => continue,
                    Some(img) => {
                        if matches!(self.images[img as usize], Image::External(_)) {
                            continue;
                        }
                        &mut tilemap_uses[img as usize]
                    }
                };
                let tiles = l.tiles.unwrap_ref();
                for tile in tiles {
                    let id = tile.id as usize;
                    mapres[id / 16][id % 16] = true;
                }
            }
        }
        let mut quads_uses: Vec<_> = self
            .images
            .iter()
            .map(|img| match img {
                Image::Embedded(emb) => {
                    let (width, height) = emb.image.size().az::<usize>().into_tuple();
                    Some(Array2::from_elem((height, width), false))
                }
                Image::External(_) => None,
            })
            .collect();
        for layer in self.groups.iter().flat_map(|g| g.layers.iter()) {
            let layer = match layer {
                Layer::Quads(l) => l,
                _ => continue,
            };
            let image_index = match layer.image {
                Some(i) => i,
                None => continue,
            };
            let uses = match &mut quads_uses[image_index as usize] {
                None => continue,
                Some(uses) => uses,
            };
            let (image_height, image_width) = uses.dim();
            let image_dim: Vec2<usize> = Vec2::new(image_width, image_height);
            let image_dim_i64: Vec2<i64> = image_dim.az();
            let image_dim_f: Vec2<I32F0> = image_dim.map(|v| I32F0::checked_from_num(v).unwrap());
            for quad in &layer.quads {
                let texture_corners: [Vec2<i64>; 4] = quad.texture_coords.map(|uv| {
                    let uv = Vec2::new(uv.u, uv.v);
                    let uv = uv.map2(image_dim_f, |a, b| a.wide_mul(b));
                    uv.map(|x| x.to_num())
                });
                for indices in [[0, 1, 3], [0, 2, 3]] {
                    let [a, b, c] = indices.map(|i| texture_corners[i]);
                    let rasterizer = Rasterizer::new(a, b, c);
                    for point in rasterizer {
                        let clipped = point.map2(image_dim_i64, |a, b| a.rem_euclid(b) as usize);
                        uses[(clipped.y, clipped.x)] = true;
                    }
                }
            }
        }

        for ((mapres, tm), mut qu) in self.images.iter_mut().zip(tilemap_uses).zip(quads_uses) {
            let embedded = match mapres {
                Image::External(_) => continue,
                Image::Embedded(emb) => emb,
            };
            let quads_uses = qu.as_mut().unwrap();
            let mut quads_uses_tmp = quads_uses.clone();
            for _ in 0..3 {
                widen(quads_uses, &mut quads_uses_tmp);
                widen(&quads_uses_tmp, quads_uses);
            }
            let rgba = embedded.image.unwrap_mut();
            let tile_width = rgba.width() as usize / 16;
            let tm_inactive = !tm.iter().any(|row| row.iter().any(|b| *b));
            for (x, y, pixel) in rgba.enumerate_pixels_mut() {
                if !quads_uses[(y as usize, x as usize)]
                    && (tm_inactive || !tm[y as usize / tile_width][x as usize / tile_width])
                {
                    *pixel = image::Rgba([0, 0, 0, 0]);
                }
            }
            embedded.dilate();
        }
    }
}

fn widen(src: &Array2<bool>, dst: &mut Array2<bool>) {
    let (height, width) = src.dim();
    assert_eq!(dst.dim(), (height, width));
    for (((y, x), src_b), dst_b) in src.indexed_iter().zip(dst.iter_mut()) {
        *dst_b = *src_b
            || [(0, 1), (1, 0), (0, -1), (-1, 0)].iter().any(|(dy, dx)| {
                let y = (y as isize + dy).rem_euclid(height as isize) as usize;
                let x = (x as isize + dx).rem_euclid(width as isize) as usize;
                src[(y, x)]
            })
    }
}

struct Rasterizer {
    a: Vec2<i64>,
    b: Vec2<i64>,
    c: Vec2<i64>,
    min: Vec2<i64>,
    max: Vec2<i64>,
    current: Vec2<i64>,
}

impl Rasterizer {
    fn new(a: Vec2<i64>, mut b: Vec2<i64>, mut c: Vec2<i64>) -> Self {
        let min = a.map3(b, c, |a, b, c| a.min(b.min(c)));
        let max = a.map3(b, c, |a, b, c| a.max(b.max(c)));
        let current = min;
        if a.determine_side(b, c) < 0 {
            std::mem::swap(&mut b, &mut c);
        }
        Self {
            a,
            b,
            c,
            min,
            max,
            current,
        }
    }
}

impl Iterator for Rasterizer {
    type Item = Vec2<i64>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let current = self.current;
            if current.y > self.max.y {
                break None;
            }
            let mut next = self.current;
            next.x += 1;
            if next.x > self.max.x {
                next.x = self.min.x;
                next.y += 1;
            }
            self.current = next;
            if current.determine_side(self.a, self.b) >= 0
                && current.determine_side(self.b, self.c) >= 0
                && current.determine_side(self.c, self.a) >= 0
            {
                break Some(current);
            }
        }
    }
}

#[test]
fn rasterizer() {
    use std::collections::HashSet;
    let expected = vec![
        Vec2 { x: 0, y: 0 },
        Vec2 { x: 1, y: 0 },
        Vec2 { x: 2, y: 0 },
        Vec2 { x: 3, y: 0 },
        Vec2 { x: 0, y: 1 },
        Vec2 { x: 1, y: 1 },
        Vec2 { x: 2, y: 1 },
        Vec2 { x: 0, y: 2 },
        Vec2 { x: 1, y: 2 },
        Vec2 { x: 0, y: 3 },
    ]
    .into_iter()
    .collect::<HashSet<Vec2<i64>>>();
    let points = [Vec2::new(0, 0), Vec2::new(0, 3), Vec2::new(3, 0)];
    for permutation in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let [a, b, c] = permutation.map(|i| points[i]);
        let rasterizer = Rasterizer::new(a, b, c);
        let points = rasterizer.collect::<HashSet<_>>();
        assert_eq!(
            points,
            expected,
            "missing: {:?} in permutation {permutation:?}",
            expected.difference(&points),
        );
    }
}

use crate::EmbeddedImage;
use image::RgbaImage;

const ALPHA_THRESHOLD: u8 = 10;

impl EmbeddedImage {
    /// Panics, if the image is not loaded
    pub fn dilate(&mut self) {
        dilate(self.image.unwrap_mut())
    }
}

pub fn dilate(image: &mut RgbaImage) {
    let mut tmp1 = RgbaImage::new(image.width(), image.height());
    let mut tmp2 = tmp1.clone();
    dilate_helper(image, &mut tmp1);
    for _ in 0..5 {
        dilate_helper(&tmp1, &mut tmp2);
        dilate_helper(&tmp2, &mut tmp1);
    }
    image
        .pixels_mut()
        .zip(tmp1.pixels())
        .for_each(|(original, dilated)| original.0[..3].copy_from_slice(&dilated.0[..3]))
}

fn dilate_helper(src: &RgbaImage, dst: &mut RgbaImage) {
    let (width, height) = (src.width(), src.height());
    assert_eq!((width, height), (dst.width(), dst.height()));

    for ((x, y, src_pixel), dst_pixel) in src.enumerate_pixels().zip(dst.pixels_mut()) {
        *dst_pixel = *src_pixel;
        if src_pixel.0[3] > ALPHA_THRESHOLD {
            continue;
        }
        let mut neighbor_sum = vek::Rgb::<u32>::from(0);
        let mut neighbor_count = 0;

        const DIRECTIONS: [(i64, i64); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];
        for direction in DIRECTIONS {
            let x_ = (x as i64 + direction.0).clamp(0, width as i64 - 1);
            let y_ = (y as i64 + direction.1).clamp(0, height as i64 - 1);
            let neighbor = vek::Rgba::<u8>::from(src.get_pixel(x_ as u32, y_ as u32).0);
            if neighbor.a > ALPHA_THRESHOLD {
                neighbor_sum += neighbor.rgb().az();
                neighbor_count += 1;
            }
        }
        if neighbor_count > 0 {
            let new_rgb = (neighbor_sum / neighbor_count).az::<u8>();
            let new_pixel = vek::Rgba::from_opaque(new_rgb);
            dst_pixel
                .0
                .copy_from_slice(new_pixel.into_array().as_slice());
        }
    }
}

use egui::Rect as EGuiRect;
use macroquad::camera::Camera2D;
use macroquad::color::colors::*;
use macroquad::math::{Rect, Vec2};
use macroquad::shapes::{draw_circle, draw_line, draw_rectangle_lines};
use macroquad::window::{screen_height, screen_width};

const ZOOM_FACTOR: f32 = 0.9;

pub struct MapCamera {
    offset: Vec2,
    zoom: f32,
    map_width: f32,
    map_height: f32,
    viewport: Vec2,
}

impl MapCamera {
    pub fn new() -> MapCamera {
        MapCamera {
            offset: Vec2::ZERO,
            zoom: 1.0,

            // TODO: i guess it would be cleaner to use options here :3
            map_width: 0.,
            map_height: 0.,
            viewport: Vec2::ZERO,
        }
    }

    pub fn update_map_size(&mut self, map_width: usize, map_height: usize) {
        self.map_width = map_width as f32;
        self.map_height = map_height as f32;
    }

    pub fn update_viewport_from_egui_rect(&mut self, canvas: &EGuiRect) {
        self.viewport = Vec2::new(canvas.max.x, canvas.max.y);
    }

    /// reset camera transformations
    pub fn reset(&mut self) {
        self.offset = Vec2::ZERO;
        self.zoom = 1.0;
    }

    /// zooms in or out by some constant zoom factor
    pub fn zoom(&mut self, zoom_in: bool) {
        match zoom_in {
            true => self.zoom /= ZOOM_FACTOR,
            false => self.zoom *= ZOOM_FACTOR,
        }
    }

    /// expects a "local" shift in [-1, +1] range wrt. to the full window size.
    /// if a viewport is used it will be scaled accordingly.
    pub fn shift(&mut self, local_shift: Vec2) {
        let x_ratio = self.viewport.x / screen_width();
        let y_ratio = self.viewport.y / screen_height();
        let local_shift = Vec2::new(local_shift.x / x_ratio, local_shift.y / y_ratio);

        self.offset += local_shift / self.zoom;
    }

    pub fn get_macroquad_cam(&self) -> Camera2D {
        // Calculate aspect ratio
        let viewport_ratio = self.viewport.x / self.viewport.y;
        let map_ratio = self.map_width / self.map_height;
        let (cam_width, cam_height) = if viewport_ratio > map_ratio {
            (self.map_width * viewport_ratio / map_ratio, self.map_height)
        } else {
            (self.map_width, self.map_height * map_ratio / viewport_ratio)
        };

        // set camera rect
        let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, cam_width, cam_height));

        // apply user transformations
        cam.target = Vec2::new(
            (self.offset.x / cam.zoom.x) + (cam_width / 2.),
            (-self.offset.y / cam.zoom.y) + (cam_height / 2.),
        );
        cam.zoom *= self.zoom;
        cam.viewport = Some((0, 0, self.viewport.x as i32, self.viewport.y as i32));

        cam
    }

    /// debug draws
    pub fn draw_cam_debug(&self, cam: &Camera2D) {
        // doesnt work due to viewport
        // let mouse_abs = cam.screen_to_world(Vec2::new(mouse_position().0, mouse_position().1));
        // draw_circle(mouse_abs.x, mouse_abs.y, 1.0, LIME);

        draw_line(0.0, 0.0, self.map_width, self.map_height, 2., BLUE);
        draw_rectangle_lines(0.0, 0.0, self.map_width, self.map_height, 2.0, RED);
        draw_circle(self.map_width / 2., self.map_height / 2., 2.0, LIME);
        draw_circle(cam.target.x, cam.target.y, 2.0, DARKBLUE);
    }
}

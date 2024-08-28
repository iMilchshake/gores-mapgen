use egui::Rect as EGuiRect;
use macroquad::camera::Camera2D;
use macroquad::color::colors::*;
use macroquad::input::mouse_position;
use macroquad::math::{Rect, Vec2};
use macroquad::shapes::{draw_circle, draw_line, draw_rectangle_lines};
use macroquad::window::{screen_height, screen_width};

const ZOOM_FACTOR: f32 = 0.9;

pub struct MapCamera {
    offset: Vec2,
    zoom: f32,
    map_size: Option<Vec2>,
    viewport: Option<Vec2>,
    viewport_ratio: Option<Vec2>,
    cam: Option<Camera2D>,
}

impl Default for MapCamera {
    fn default() -> MapCamera {
        MapCamera {
            offset: Vec2::ZERO,
            zoom: 1.0,
            map_size: None,
            viewport: None,
            viewport_ratio: None,
            cam: None,
        }
    }
}

impl MapCamera {
    pub fn update_map_size(&mut self, map_width: usize, map_height: usize) {
        self.map_size = Some(Vec2::new(map_width as f32, map_height as f32));
    }

    pub fn update_viewport_from_egui_rect(&mut self, canvas: &EGuiRect) {
        let viewport = Vec2::new(canvas.max.x, canvas.max.y);
        self.viewport_ratio = Some(Vec2::new(
            viewport.x / screen_width(),
            viewport.y / screen_height(),
        ));
        self.viewport = Some(viewport);
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
        let viewport_ratio = self.viewport_ratio.expect("viewport not defined");
        let local_shift = local_shift / viewport_ratio;
        self.offset += local_shift / self.zoom;
    }

    pub fn update_macroquad_cam(&mut self) {
        let viewport = self.viewport.expect("viewport not defined!");
        let map_size = self.map_size.expect("map size not defined!");

        // Calculate aspect ratio
        let viewport_ratio = viewport.x / viewport.y;
        let map_ratio = map_size.x / map_size.y;
        let (cam_width, cam_height) = if viewport_ratio > map_ratio {
            (map_size.x * viewport_ratio / map_ratio, map_size.y)
        } else {
            (map_size.x, map_size.y * map_ratio / viewport_ratio)
        };

        // set camera rect
        let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, cam_width, cam_height));

        // apply user transformations
        cam.target = Vec2::new(
            (self.offset.x / cam.zoom.x) + (cam_width / 2.),
            (-self.offset.y / cam.zoom.y) + (cam_height / 2.),
        );
        cam.zoom *= self.zoom;
        cam.viewport = Some((0, 0, viewport.x as i32, viewport.y as i32));

        macroquad::camera::set_camera(&cam);
        self.cam = Some(cam);
    }

    pub fn get_macroquad_cam(&self) -> &Camera2D {
        self.cam.as_ref().unwrap()
    }

    pub fn get_map_mouse_pos(&self) -> Vec2 {
        let viewport_ratio = self.viewport_ratio.expect("viewport not defined");
        let cam = self.cam.expect("macroquad cam not defined");

        let mouse_pos = mouse_position();
        let mouse_viewport_pos = Vec2::new(mouse_pos.0, mouse_pos.1) / viewport_ratio;
        

        cam.screen_to_world(mouse_viewport_pos)
    }

    /// debug draws
    pub fn draw_cam_debug(&self) {
        let map_size = self.map_size.expect("map size not defined!");
        let cam = self.cam.expect("macroquad cam not defined");

        draw_line(0.0, 0.0, map_size.x, map_size.y, 2., BLUE);
        draw_rectangle_lines(0.0, 0.0, map_size.x, map_size.y, 2.0, RED);
        draw_circle(map_size.x / 2., map_size.y / 2., 2.0, LIME);
        draw_circle(cam.target.x, cam.target.y, 2.0, DARKBLUE);
    }
}

use macroquad::math::Rect;
use macroquad::prelude::*;

// TODO: this approach works rather well, but is has one major downside:
// the viewport has the same aspect ratio as the map which allows
// for rather easy mapping from map-space to camera/screen space
// but this also means for larger discrepancies between window
// and map aspect ratios a lot of screen space might be unused..
// ---
// Possible approach: Still use entire available screen space as viewport, but
// add some kind of scaling factors to deal with the distortion?

const MAP_WIDTH: f32 = 400.0;
const MAP_HEIGHT: f32 = 400.0;
const ASPECT_RATIO: f32 = MAP_WIDTH / MAP_HEIGHT;
const ZOOM_FACTOR: f32 = 0.5;
const SHIFT_FACTOR: f32 = 0.1;

struct Editor {
    zoom: f32,
    offset: Vec2,
}

impl Editor {
    fn get_display_factor() -> f32 {
        f32::min(screen_width() / MAP_WIDTH, screen_height() / MAP_HEIGHT)
    }

    /// this should result in the exact same behaviour as if not using a camera at all
    fn reset_camera() {
        set_camera(&Camera2D::from_display_rect(Rect::new(
            0.0,
            screen_height(),
            screen_width(),
            -screen_height(),
        )));
    }

    fn handle_user_inputs(&mut self) {
        if is_key_released(KeyCode::E) {
            self.zoom *= ZOOM_FACTOR;
        } else if is_key_released(KeyCode::Q) {
            self.zoom /= ZOOM_FACTOR;
        } else if is_key_released(KeyCode::R) {
            *self = Editor::default();
        } else if is_key_released(KeyCode::D) {
            self.offset.x += SHIFT_FACTOR;
        } else if is_key_released(KeyCode::A) {
            self.offset.x -= SHIFT_FACTOR;
        } else if is_key_released(KeyCode::W) {
            self.offset.y += SHIFT_FACTOR;
        } else if is_key_released(KeyCode::S) {
            self.offset.y -= SHIFT_FACTOR;
        }
    }

    fn set_cam(&self) -> Camera2D {
        let display_factor = Editor::get_display_factor();
        let x_view = display_factor * MAP_WIDTH;
        let y_view = display_factor * MAP_HEIGHT;

        let y_shift = screen_height() - y_view;
        let mut cam =
            Camera2D::from_display_rect(Rect::new(0.0, MAP_HEIGHT, MAP_WIDTH, -MAP_HEIGHT));

        cam.viewport = Some((0, y_shift as i32, x_view as i32, y_view as i32));

        cam.offset = self.offset;
        cam.zoom *= self.zoom;

        set_camera(&cam);

        cam
    }
}

impl Default for Editor {
    fn default() -> Editor {
        Editor {
            zoom: 1.0,
            offset: Vec2::ZERO,
        }
    }
}

#[macroquad::main("Camera")]
async fn main() {
    let mut editor = Editor::default();

    loop {
        editor.handle_user_inputs();
        let cam = Editor::set_cam(&editor);

        clear_background(LIGHTGRAY);

        // draw inside canvas/map
        draw_line(0.0, 0.0, MAP_WIDTH, MAP_HEIGHT, 1.0, BLUE);
        draw_circle(0.0, 0.0, 5.0, RED);
        draw_circle(MAP_WIDTH / 2., MAP_HEIGHT / 2., 5.0, RED);
        draw_circle(MAP_WIDTH, MAP_HEIGHT, 5.0, YELLOW);
        draw_rectangle_lines(0.0, 0.0, MAP_WIDTH, MAP_HEIGHT, 5.0, BLACK);

        Editor::reset_camera();

        // draw globally
        draw_circle(0.0, 0.0, 15.0, ORANGE);
        draw_circle(screen_width(), screen_height(), 15.0, ORANGE);
        draw_line(0.0, 0.0, screen_width(), screen_height(), 1.0, BLUE);
        draw_rectangle_lines(
            cam.viewport().unwrap().0 as f32,
            cam.viewport().unwrap().1 as f32,
            cam.viewport().unwrap().2 as f32,
            cam.viewport().unwrap().3 as f32,
            5.0,
            BLACK,
        );

        next_frame().await
    }
}

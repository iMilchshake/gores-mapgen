use macroquad::math::Rect;
use macroquad::prelude::*;
use rand_distr::num_traits::{Signed, Zero};

// TODO: this approach works rather well, but is has one major downside:
// the viewport has the same aspect ratio as the map which allows
// for rather easy mapping from map-space to camera/screen space
// but this also means for larger discrepancies between window
// and map aspect ratios a lot of screen space might be unused..
// ---
// Possible approach: Still use entire available screen space as viewport, but
// add some kind of scaling factors to deal with the distortion?

const MAP_WIDTH: f32 = 600.0;
const MAP_HEIGHT: f32 = 600.0;
const ZOOM_FACTOR: f32 = 0.9;

struct Editor {
    zoom: f32,
    offset: Vec2,
}

impl Editor {
    fn get_display_factor() -> f32 {
        f32::min(screen_width() / MAP_WIDTH, screen_height() / MAP_HEIGHT)
    }

    fn mouse_in_viewport(cam: &Camera2D) -> bool {
        let (mouse_x, mouse_y) = mouse_position();
        0.0 <= mouse_x
            && mouse_x <= cam.viewport.unwrap().2 as f32
            && 0.0 <= mouse_y
            && mouse_y <= cam.viewport.unwrap().3 as f32
    }

    /// this should result in the exact same behaviour as if not using a camera at all
    fn reset_camera() {
        // no idea why i dont have to use negative values here???
        set_camera(&Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
        )));
    }

    fn set_cam(&self) -> Camera2D {
        let display_factor = Editor::get_display_factor();
        let x_view = display_factor * MAP_WIDTH;
        let y_view = display_factor * MAP_HEIGHT;
        let y_shift = screen_height() - y_view;
        let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, MAP_WIDTH, MAP_HEIGHT));

        // so i guess this is (x, y, width, height) not two positions?
        cam.viewport = Some((0, y_shift as i32, x_view as i32, y_view as i32));

        cam.target -= self.offset;
        cam.zoom *= self.zoom;

        set_camera(&cam);

        cam
    }

    fn handle_mouse_inputs(&mut self) {
        // handle mouse inputs
        let mouse_wheel_y = mouse_wheel().1;
        if !mouse_wheel_y.is_zero() {
            if mouse_wheel_y.is_positive() {
                self.zoom /= ZOOM_FACTOR;
            } else {
                self.zoom *= ZOOM_FACTOR;
            }
        }
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

    let mut last_mouse: Option<Vec2> = None;

    loop {
        editor.handle_mouse_inputs();

        let cam = Editor::set_cam(&editor);

        if is_mouse_button_down(MouseButton::Left) && Editor::mouse_in_viewport(&cam) {
            let mouse = mouse_position();

            if let Some(last_mouse) = last_mouse {
                let display_factor = Editor::get_display_factor();
                let local_delta = Vec2::new(mouse.0, mouse.1) - last_mouse;
                editor.offset += local_delta / (editor.zoom * display_factor);
            }

            last_mouse = Some(mouse.into());

        // mouse pressed for first frame, reset last position
        } else if is_mouse_button_released(MouseButton::Left) {
            last_mouse = None;
        }

        clear_background(LIGHTGRAY);

        // draw inside canvas/map
        draw_line(0.0, 0.0, MAP_WIDTH, MAP_HEIGHT, 1.0, BLUE);
        draw_circle(0.0, 0.0, 5.0, RED);
        draw_circle(MAP_WIDTH / 2., MAP_HEIGHT / 2., 5.0, RED);
        draw_circle(MAP_WIDTH, MAP_HEIGHT, 5.0, YELLOW);
        draw_rectangle_lines(0.0, 0.0, MAP_WIDTH, MAP_HEIGHT, 5.0, BLACK);

        // draw target
        draw_circle(cam.target.x, cam.target.y, 2.5, ORANGE);

        Editor::reset_camera();

        // draw globally
        draw_circle(0.0, 0.0, 15.0, ORANGE);
        draw_circle(screen_width(), screen_height(), 15.0, ORANGE);
        draw_line(0.0, 0.0, screen_width(), screen_height(), 1.0, BLUE);
        draw_rectangle_lines(
            0.0,
            0.0,
            cam.viewport().unwrap().2 as f32,
            cam.viewport().unwrap().3 as f32,
            5.0,
            BLACK,
        );

        let mouse_pos_abs = mouse_position();
        draw_circle_lines(mouse_pos_abs.0, mouse_pos_abs.1, 10.0, 2.0, GRAY);

        next_frame().await
    }
}

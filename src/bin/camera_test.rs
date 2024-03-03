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

const MAP_WIDTH: f32 = 600.0;
const MAP_HEIGHT: f32 = 600.0;
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

        cam.target.x -= self.offset.x;
        cam.target.y -= self.offset.y;
        cam.zoom *= self.zoom;

        set_camera(&cam);

        cam
    }

    fn handle_user_inputs(&mut self) {
        if is_key_pressed(KeyCode::Q) {
            self.zoom *= ZOOM_FACTOR;
        } else if is_key_pressed(KeyCode::E) {
            self.zoom /= ZOOM_FACTOR;
        } else if is_key_pressed(KeyCode::R) {
            *self = Editor::default();
        } else if is_key_pressed(KeyCode::A) {
            self.offset.x += SHIFT_FACTOR;
        } else if is_key_pressed(KeyCode::D) {
            self.offset.x -= SHIFT_FACTOR;
        } else if is_key_pressed(KeyCode::W) {
            self.offset.y += SHIFT_FACTOR;
        } else if is_key_pressed(KeyCode::S) {
            self.offset.y -= SHIFT_FACTOR;
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
    let mut last_local_mouse: Option<Vec2> = None;

    loop {
        editor.handle_user_inputs();

        let cam = Editor::set_cam(&editor);

        // TODO: yeah no this doesnt work. i need to calculate my own delta completly disregarding
        // the camera, because its offset will fuck everything up. so i guess i should just look at
        // the global position and somehow scale it using the actual viewport?
        if is_mouse_button_down(MouseButton::Left) && Editor::mouse_in_viewport(&cam) {
            let current_local_mouse = cam.screen_to_world(mouse_position().into());

            if let Some(last_local_m) = last_local_mouse {
                let local_delta = current_local_mouse - last_local_m;

                editor.offset += local_delta;

                dbg!((current_local_mouse, local_delta));
            }

            last_local_mouse = Some(current_local_mouse);
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

        let mouse_pos_abs = mouse_position();
        let mouse_viewport = cam.screen_to_world(mouse_pos_abs.into());

        draw_circle_lines(mouse_viewport.x, mouse_viewport.y, 8.0, 2.0, GRAY);

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

        draw_circle_lines(mouse_pos_abs.0, mouse_pos_abs.1, 10.0, 2.0, GRAY);

        next_frame().await
    }
}

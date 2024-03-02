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

fn get_display_factor() -> f32 {
    f32::min(screen_width() / MAP_WIDTH, screen_height() / MAP_HEIGHT)
}

fn reset_cam() -> Camera2D {
    dbg!(ASPECT_RATIO);
    let display_factor = get_display_factor();
    let x_view = display_factor * MAP_WIDTH;
    let y_view = display_factor * MAP_HEIGHT;

    dbg!((screen_width(), screen_height()));
    dbg!((&x_view, &y_view, &display_factor));

    let y_shift = screen_height() - y_view;
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, MAP_HEIGHT, MAP_WIDTH, -MAP_HEIGHT));

    cam.viewport = Some((0, y_shift as i32, x_view as i32, y_view as i32));

    set_camera(&cam);

    cam
}

#[macroquad::main("Camera")]
async fn main() {
    let mut cam = reset_cam();

    loop {
        set_camera(&cam);

        if is_key_released(KeyCode::W) {
            cam.zoom.x *= ZOOM_FACTOR / ASPECT_RATIO;
            cam.zoom.y *= ZOOM_FACTOR;
            set_camera(&cam);
        } else if is_key_released(KeyCode::S) {
            cam.zoom.x /= ZOOM_FACTOR / ASPECT_RATIO;
            cam.zoom.y /= ZOOM_FACTOR;
            set_camera(&cam);
        } else if is_key_released(KeyCode::R) {
            cam = reset_cam(); // override
        } else if is_key_released(KeyCode::D) {
            cam.offset.x += SHIFT_FACTOR;
        } else if is_key_released(KeyCode::A) {
            cam.offset.x -= SHIFT_FACTOR;
        }

        clear_background(LIGHTGRAY);
        // lets assume drawing is from 0 - 100
        draw_line(0.0, 0.0, MAP_WIDTH, MAP_HEIGHT, 1.0, BLUE);
        draw_circle(0.0, 0.0, 5.0, RED);
        draw_circle(cam.target.x, cam.target.y, 5.0, RED);
        draw_circle(MAP_WIDTH, MAP_HEIGHT, 5.0, YELLOW);
        draw_rectangle_lines(0.0, 0.0, MAP_WIDTH, MAP_HEIGHT, 1.0, BLACK);

        // draw_rectangle_lines(-1.0, -1.0, 2.0, 2.0, 0.01, RED);

        // Back to screen space, render some text

        // set_camera(&amera2D::default());

        // set_default_camera();

        set_camera(&Camera2D::from_display_rect(Rect::new(
            0.0,
            screen_height(),
            screen_width(),
            -screen_height(),
        )));
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

        // draw_rectangle_lines(0.0, 0.0, MAP_WIDTH, MAP_HEIGHT, 1.0, GREEN);

        next_frame().await
    }
}

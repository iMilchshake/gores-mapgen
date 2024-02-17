mod grid_test;

use grid_test::*;


use array2d::*;

use egui::{epaint::Shadow, Color32, Frame, Label, Margin, Rect};
use macroquad::prelude::*;

const LEVEL_SIZE: usize = 500;
const SHIFT_FACTOR: f32 = 250.0;
const ZOOM_FACTOR: f32 = 1.1;

fn window_frame() -> Frame {
    Frame {
        fill: Color32::from_gray(0),
        inner_margin: Margin::same(5.0),
        shadow: Shadow::NONE,
        ..Default::default()
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "egui with macroquad".to_owned(),
        ..Default::default()
    }
}


#[macroquad::main(window_conf)]
async fn main() {
    let mut main_rect: Rect = Rect::EVERYTHING;

    let mut display_factor: f32 = 1.0;
    let mut display_shift: Vec2 = vec2(10.0, 10.0);

    let mut grid: Array2D<BlockType> =
        Array2D::filled_with(BlockType::Empty, LEVEL_SIZE, LEVEL_SIZE);
    for _ in 1..5500 {
        let point = Vec2D::random_pos();
        grid.set(point.x, point.y, BlockType::Filled).unwrap();
    }

    loop {
        clear_background(WHITE);

        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label("hello world");
                ui.separator();

                if ui.button("YEET").clicked() {
                    grid = Array2D::filled_with(BlockType::Empty, LEVEL_SIZE, LEVEL_SIZE);
                    for _ in 1..5500 {
                        let point = Vec2D::random_pos();
                        grid.set(point.x, point.y, BlockType::Filled).unwrap();
                    }
                };

            });

            egui::Window::new("yeah").frame(window_frame()).show(egui_ctx, |ui| {
                ui.add(Label::new("this is some UI stuff"));
                ui.button("text").clicked();
            });

            main_rect = egui_ctx.available_rect();
        });

        if main_rect.contains(mouse_position().into()) {
            handle_mouse_inputs(&mut display_factor, &mut display_shift);
        }

        draw_grid_blocks(&grid, display_factor, display_shift);
        macroquad::models::draw_grid(10, 10.0, BLACK, GREEN);

        egui_macroquad::draw();
        next_frame().await
    }
}

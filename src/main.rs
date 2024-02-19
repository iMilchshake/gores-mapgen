mod grid_test;

use grid_test::*;

use ndarray::Array2;

use egui::{epaint::Shadow, Color32, Frame, Label, Margin, Rect};
use macroquad::prelude::*;

const LEVEL_SIZE: usize = 100;

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

fn min(x: f32, y: f32) -> f32 {
    if x < y {
        return x;
    } else {
        return y;
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut main_rect: Rect = Rect::EVERYTHING;
    let mut grid = Array2::from_elem((LEVEL_SIZE, LEVEL_SIZE), BlockType::Empty);

    for _ in 1..500 {
        let point = Vec2D::random_pos(LEVEL_SIZE);
        grid[[point.x, point.y]] = BlockType::Filled;
    }

    loop {
        clear_background(WHITE);

        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label("hello world");
                ui.separator();
            });

            egui::Window::new("yeah")
                .frame(window_frame())
                .show(egui_ctx, |ui| {
                    ui.add(Label::new("this is some UI stuff"));
                    ui.button("text").clicked();
                });

            main_rect = egui_ctx.available_rect();
        });

        // TODO: add proper mouse input xd
        // if main_rect.contains(mouse_position().into()) {
        //     handle_mouse_inputs(&mut display_factor, &mut display_shift);
        // }

        let available_length = min(main_rect.width(), main_rect.height()); // TODO: assumes square
        let display_factor = available_length / LEVEL_SIZE as f32;

        draw_grid_blocks(&grid, display_factor, vec2(0.0, 0.0));

        egui_macroquad::draw();
        next_frame().await
    }
}

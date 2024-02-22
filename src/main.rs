mod grid_render;
mod map;
mod position;
mod walker;

use std::{borrow::Borrow, usize};

use grid_render::*;
use map::*;
use position::*;
use walker::*;

use egui::{
    epaint::{ahash::random_state, Shadow},
    Color32, Frame, Label, Margin, Rect,
};
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

// TODO: not quite sure where to put this, this doesnt
// have any functionality, so a seperate file feels overkill
#[derive(Debug)]
pub enum ShiftDirection {
    Up,
    Right,
    Down,
    Left,
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut canvas: Rect = Rect::EVERYTHING;
    let mut map = Map::new(LEVEL_SIZE, LEVEL_SIZE, BlockType::Empty);
    let mut walker = CuteWalker::new(Position::new(50, 50));

    let kernel = Kernel::new(9, 1.0);

    // setup waypoints
    let goals: Vec<Position> = vec![
        Position::new(99, 50),
        Position::new(0, 50),
        Position::new(50, 50),
        Position::new(50, 99),
        Position::new(50, 0),
    ];
    let mut goals_iter = goals.iter();
    let mut curr_goal = goals_iter.next().unwrap();

    // very important
    walker.cuddle();

    loop {
        clear_background(WHITE);

        // walker logic
        if walker.pos.ne(&curr_goal) {
            let shift = walker.pos.get_greedy_dir(&curr_goal);
            walker
                .shift_pos(shift, &map)
                .expect("Expecting valid shift here");
            map.update(&walker.pos, &kernel, BlockType::Filled)
                .unwrap_or_else(|_| {
                    println!("bounds exceeded :))");
                });
        } else if let Some(next_goal) = goals_iter.next() {
            curr_goal = next_goal;
        }

        // define egui
        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label("hello world");
                ui.separator();
            });

            egui::Window::new("DEBUG")
                .frame(window_frame())
                .show(egui_ctx, |ui| {
                    ui.add(Label::new(get_fps().to_string()));
                    ui.add(Label::new(format!("{:?}", walker)));
                    ui.add(Label::new(format!("{:?}", curr_goal)));
                });

            // store remaining space for macroquad drawing
            canvas = egui_ctx.available_rect();
        });

        // draw grid
        let display_factor = (f32::min(canvas.width(), canvas.height())) / LEVEL_SIZE as f32; // TODO: assumes square
        draw_grid_blocks(&mut map.grid, display_factor, vec2(0.0, 0.0));
        draw_walker(&walker, display_factor, vec2(0.0, 0.0));

        // draw egui on top of macroquad
        egui_macroquad::draw();

        next_frame().await
    }
}

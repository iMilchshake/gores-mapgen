mod grid_render;
mod map;
mod position;
mod walker;

use std::usize;

use grid_render::*;
use map::*;
use position::*;

use notan::draw::*;
use notan::egui::{self, *};
use notan::math::Vec2;
use notan::prelude::*;
use walker::*;

// TODO: not quite sure where to put this, this doesnt
// have any functionality, so a seperate file feels overkill
#[derive(Debug)]
pub enum ShiftDirection {
    Up,
    Right,
    Down,
    Left,
}

#[derive(AppState)]
struct State {
    canvas: Rect,
    map: Map,
    walker: CuteWalker,
    kernel: Kernel,
    pause: bool,
    allowed_step: usize,
    goals: Vec<Position>,
    goal_index: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            canvas: Rect::EVERYTHING,
            map: Map::new(100, 100, BlockType::Empty),
            walker: CuteWalker::new(Position::new(50, 33)),
            pause: false,
            allowed_step: 0,
            goals: vec![
                Position::new(99, 33),
                Position::new(0, 33),
                Position::new(50, 33),
                Position::new(50, 100),
            ],
            goal_index: 0,
            kernel: Kernel::new(5, 1.0),
        }
    }
}

#[notan_main]
fn main() -> Result<(), String> {
    let win_config = WindowConfig::new().set_vsync(true).set_resizable(true);

    // .set_size(WIDTH, HEIGHT)
    // .set_multisampling(8)
    // .set_lazy_loop(true)
    // .set_high_dpi(true);

    notan::init_with(State::default)
        .add_config(win_config)
        .add_config(EguiConfig)
        .add_config(DrawConfig)
        .draw(draw)
        .update(update)
        .build()
}
// TODO: very important
// walker.cuddle();

fn draw(gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
    let egui_output = plugins.egui(|ctx| {
        // Draw the EGUI Widget here
        draw_egui_widget(ctx, state);
    });

    let display_factor = f32::min(
        state.canvas.width() / state.map.width as f32,
        state.canvas.height() / state.map.height as f32,
    );

    // draw_walker(&walker, display_factor, vec2(0.0, 0.0));

    // Draw shape
    let mut draw = gfx.create_draw();
    draw.clear(Color::WHITE);
    draw_grid_blocks(
        &mut draw,
        &state.map.grid,
        display_factor,
        Vec2::new(0.0, 0.0),
    );
    gfx.render(&draw);

    // Draw the context to the screen or to a RenderTexture
    gfx.render(&egui_output);
}

fn draw_egui_widget(ctx: &egui::Context, state: &mut State) {
    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        ui.label("hello world");

        // toggle pause
        if ui.button("toggle").clicked() {
            state.pause = !state.pause;
        }

        // pause, allow single step
        if ui.button("single").clicked() {
            state.pause = true;
            state.allowed_step += 1;
        }
        ui.separator();
    });

    // egui::Window::new("DEBUG")
    //     .frame(window_frame())
    //     .show(ctx, |ui| {
    //         ui.add(Label::new(format!("fps: {:}", get_fps().to_string())));
    //         ui.add(Label::new(format!(
    //             "allowed_step: {:}",
    //             allowed_step.to_string()
    //         )));
    //         ui.add(Label::new(format!("{:?}", walker)));
    //         ui.add(Label::new(format!("{:?}", curr_goal)));
    //     });

    state.canvas = ctx.available_rect()
}

fn update(app: &mut App, state: &mut State) {
    let mut curr_goal = state.goals.get(state.goal_index).unwrap();

    // if goal is reached
    if state.walker.pos.eq(curr_goal) {
        state.goal_index += 1;
        curr_goal = state.goals.get(state.goal_index).unwrap();
    }

    if !state.pause {
        state.allowed_step += 1;
    }

    if state.walker.steps < state.allowed_step {
        // get greedy shift towards goal
        let shift = state.walker.pos.get_greedy_dir(curr_goal);

        // apply that shift
        state
            .walker
            .shift_pos(shift, &state.map)
            .unwrap_or_else(|_| {
                println!("walker exceeded bounds, pausing...");
                state.pause = true;
                state.allowed_step -= 1;
            });

        // remove blocks using a kernel at current position
        state
            .map
            .update(&state.walker.pos, &state.kernel, BlockType::Filled)
            .ok();
    }
}

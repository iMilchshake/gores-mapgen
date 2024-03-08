mod editor;
mod fps_control;
mod grid_render;
mod map;
mod position;
mod random;
mod walker;
use std::f32::consts::SQRT_2;

use crate::{editor::*, fps_control::*, grid_render::*, map::*, position::*, random::*, walker::*};

use egui::Label;
use macroquad::color::*;
use macroquad::shapes::*;
use macroquad::window::clear_background;

pub fn define_egui(editor: &mut Editor, state: &mut State) {
    // define egui
    egui_macroquad::ui(|egui_ctx| {
        egui::Window::new("DEBUG")
            .frame(window_frame())
            .show(egui_ctx, |ui| {
                ui.add(Label::new(format!("TEST")));

                let inner_radius_bounds = Kernel::get_valid_radius_bounds(state.inner_size);
                let outer_radius_bounds = Kernel::get_valid_radius_bounds(state.outer_size);

                ui.add(egui::Slider::new(&mut state.inner_size, 1..=19).text("inner_size"));
                ui.add(
                    egui::Slider::new(
                        &mut state.inner_radius,
                        inner_radius_bounds.0..=inner_radius_bounds.1,
                    )
                    .text("inner_radius"),
                );
                ui.add(egui::Slider::new(&mut state.outer_size, 1..=19).text("outer_size"));
                ui.add(
                    egui::Slider::new(
                        &mut state.outer_radius,
                        outer_radius_bounds.0..=outer_radius_bounds.1,
                    )
                    .text("outer_radius"),
                );
            });

        // store remaining space for macroquad drawing
        editor.canvas = Some(egui_ctx.available_rect());
        editor.egui_wants_mouse = Some(egui_ctx.wants_pointer_input());
    });
}

struct State {
    inner_radius: f32,
    inner_size: usize,
    outer_radius: f32,
    outer_size: usize,
}

fn draw_thingy(walker: &CuteWalker, flag: bool) {
    let offset: usize = walker.kernel.size / 2; // offset of kernel wrt. position (top/left)
    let root_pos = Position::new(walker.pos.x - offset, walker.pos.y - offset);
    for ((x, y), kernel_active) in walker.kernel.vector.indexed_iter() {
        if *kernel_active {
            draw_rectangle(
                (root_pos.x + x) as f32,
                (root_pos.y + y) as f32,
                1.0,
                1.0,
                match flag {
                    true => Color::new(0.0, 1.0, 0.0, 0.5),
                    false => Color::new(1.0, 0.0, 0.0, 0.5),
                },
            );
        }
    }

    let size = walker.kernel.size;
    let radius = walker.kernel.radius;

    dbg!((&flag, radius));

    // very crappy hotfix to deal with different center whether size is even or not
    let offset = match size % 2 == 0 {
        true => 0.0,
        false => 0.5,
    };

    draw_circle_lines(
        (walker.pos.x) as f32 + offset,
        (walker.pos.y) as f32 + offset,
        radius,
        0.05,
        match flag {
            true => GREEN,
            false => RED,
        },
    );
}

// NOTE: Conclusion: a kernel pair is valid if the inner and outer radius have a difference of at
// last sqrt(2)=1.41 and a size difference of at least 2. if the size difference in larger the
// circularity constraint becomes slighly more slack, but i should be able to neglect that. Also

// NOTE: for some setups, an inner circle that should have exactly one block of padding, might be
// only possible with one exact circularity value that just hits the 1.41 difference. So in
// practice when sampling the cirtularity from a continous space, this would almost never be
// picked. I could:
// 1. Add a specific rule which will use 1-block padding with a certain probability and only
//    otherwise sample from the available circularities. (test: would different kernels have
//    different probabilities?)
// 2. Add 1-block freeze padding as in original generator?
// 3. somehow map the continous circularitiy to a discrete space. Then all configs could be sampled
//    uniform (and additionally maybe a weighting for 1-block?). One possible approach would be to
//    allow a fixed number of samples. e.g. 0.0, 0.5 and 1.0. But again, this might be good or bad
//    for different sizes. Another approach would be to pre-calculate ALL possible kernels and
//    check their compatibility? I could iterate over all sizes/circularities and then store all
//    unique kernels with their respective radius. Then, i could easily use that to determine valid
//    kernel pairs and sample from a discrete space! It might make sense to store the smallest and
//    largest radius that leads to the same kernel. This would make the validity check easier, then
//    i could use the min radius for inner and max radius for outer to make sure that they have at
//    least 1.41 distance.

// NOTE: i dont even know if all that effort would be worth it. I guess for now i should just
// enforce odd-size, size diff of 2, and min diff of 1.41 in radius and just ignore the rest for
// now xd

// TODO: first i need a way to add constraint available circularities based on valid radii.

#[macroquad::main("kernel_test")]
async fn main() {
    let mut editor = Editor::new(EditorPlayback::Paused);
    let map = Map::new(20, 20, BlockType::Hookable);

    let kernel = Kernel::new(3, 0.0);
    let mut walker = CuteWalker::new(Position::new(10, 10), vec![Position::new(15, 15)], kernel);
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);

    let mut state = State {
        inner_radius: Kernel::get_valid_radius_bounds(3).0,
        inner_size: 3,
        outer_radius: Kernel::get_valid_radius_bounds(5).1,
        outer_size: 5,
    };

    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();

        define_egui(&mut editor, &mut state);

        editor.set_cam(&map);
        editor.handle_user_inputs(&map);

        clear_background(GRAY);
        draw_walker(&walker);

        walker.kernel = Kernel::new(state.outer_size, state.outer_radius);
        draw_thingy(&walker, false);

        if (state.outer_radius - state.inner_radius) < SQRT_2 {
            state.inner_radius = state.outer_radius - SQRT_2;
        }
        walker.kernel = Kernel::new(state.inner_size, state.inner_radius);
        draw_thingy(&walker, true);

        egui_macroquad::draw();
        fps_ctrl.wait_for_next_frame().await;
    }
}

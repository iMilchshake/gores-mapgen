use gores_mapgen_rust::{
    config::GenerationConfig, editor::*, fps_control::*, kernel::*, position::*, rendering::*,
    walker::*,
};

use macroquad::color::*;
use macroquad::shapes::*;
use macroquad::window::clear_background;

fn state_to_kernels(state: &mut State) -> (Kernel, Kernel) {
    (
        Kernel::new(state.inner_size, state.inner_circ),
        Kernel::new(state.outer_size, state.outer_circ),
    )
}

fn define_egui(editor: &mut Editor, state: &mut State) {
    // define egui
    egui_macroquad::ui(|egui_ctx| {
        egui::Window::new("DEBUG")
            .frame(window_frame())
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("inner size: {}", state.inner_size));
                    if ui.button("-").clicked() {
                        state.inner_size = state.inner_size.saturating_sub(1);
                    }
                    if ui.button("+").clicked() {
                        state.inner_size = state.inner_size.saturating_add(1);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(format!("outer size: {}", state.outer_size));
                    if ui.button("-").clicked() {
                        state.outer_size = state.outer_size.saturating_sub(1);
                    }
                    if ui.button("+").clicked() {
                        state.outer_size = state.outer_size.saturating_add(1);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(format!("inner circ: {:.1}", state.inner_circ));
                    if ui.button("-").clicked() {
                        state.inner_circ = (state.inner_circ - 0.1).max(0.0);
                    }
                    if ui.button("+").clicked() {
                        state.inner_circ = (state.inner_circ + 0.1).min(1.0);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(format!("outer circ: {:.1}", state.outer_circ));
                    if ui.button("-").clicked() {
                        state.outer_circ = (state.outer_circ - 0.1).max(0.0);
                    }
                    if ui.button("+").clicked() {
                        state.outer_circ = (state.outer_circ + 0.1).min(1.0);
                    }
                });
            });

        // store remaining space for macroquad drawing
        editor.canvas = Some(egui_ctx.available_rect());
        editor.egui_wants_mouse = Some(egui_ctx.wants_pointer_input());
    });
}

#[derive(Debug)]
struct State {
    inner_circ: f32,
    outer_circ: f32,
    inner_size: usize,
    outer_size: usize,
}

fn draw_thingy(walker: &CuteWalker, flag: bool) {
    let offset: usize = walker.inner_kernel.size / 2; // offset of kernel wrt. position (top/left)
    let root_pos = Position::new(walker.pos.x - offset, walker.pos.y - offset);
    for ((x, y), kernel_active) in walker.inner_kernel.vector.indexed_iter() {
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

    let size = walker.inner_kernel.size;
    let radius = walker.inner_kernel.radius;

    // very crappy hotfix to deal with different center whether size is even or not
    let offset = match size % 2 == 0 {
        true => 0.0,
        false => 0.5,
    };

    draw_circle_lines(
        (walker.pos.x) as f32 + offset,
        (walker.pos.y) as f32 + offset,
        radius.sqrt(),
        0.05,
        match flag {
            true => GREEN,
            false => RED,
        },
    );

    draw_circle_lines(
        (walker.pos.x) as f32 + offset,
        (walker.pos.y) as f32 + offset,
        radius,
        0.025,
        match flag {
            true => GREEN,
            false => RED,
        },
    );
}

#[macroquad::main("kernel_test")]
async fn main() {
    let mut editor = Editor::new(GenerationConfig::default());
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);

    let mut state = State {
        inner_circ: 0.0,
        outer_circ: 0.0,
        inner_size: 3,
        outer_size: 5,
    };

    let (inner_kernel, outer_kernel) = state_to_kernels(&mut state);
    let mut walker = CuteWalker::new(
        Position::new(10, 10),
        inner_kernel,
        outer_kernel,
        &GenerationConfig::default(),
    );

    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();
        define_egui(&mut editor, &mut state);
        editor.set_cam();
        editor.handle_user_inputs();
        clear_background(GRAY);
        draw_walker(&walker);

        let (inner_kernel, outer_kernel) = state_to_kernels(&mut state);

        walker.inner_kernel = outer_kernel.clone();
        draw_thingy(&walker, false);

        walker.inner_kernel = inner_kernel.clone();
        draw_thingy(&walker, true);

        egui_macroquad::draw();
        fps_ctrl.wait_for_next_frame().await;
    }
}

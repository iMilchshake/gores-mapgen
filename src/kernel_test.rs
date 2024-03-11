mod editor;
mod fps_control;
mod grid_render;
mod map;
mod position;
mod random;
mod walker;

use std::f64::consts::SQRT_2;


use crate::{editor::*, fps_control::*, grid_render::*, map::*, position::*, random::*, walker::*};

use egui::emath::Numeric;
use egui::{Label};
use macroquad::color::*;
use macroquad::shapes::*;
use macroquad::window::clear_background;
use rand_distr::num_traits::ToPrimitive;

pub fn define_egui(editor: &mut Editor, state: &mut State) {
    // define egui
    egui_macroquad::ui(|egui_ctx| {
        egui::Window::new("DEBUG")
            .frame(window_frame())
            .show(egui_ctx, |ui| {
                ui.add(Label::new("TEST".to_string()));

                let inner_radius_bounds = Kernel::get_valid_radius_bounds(state.inner_size);
                let outer_radius_bounds = Kernel::get_valid_radius_bounds(state.outer_size);

                ui.add(egui::Slider::new(&mut state.inner_size, 1..=19).text("inner_size"));
                ui.add(
                    egui::Slider::new(
                        &mut state.inner_radius_sqr,
                        inner_radius_bounds.0..=inner_radius_bounds.1,
                    )
                    .text("inner_radius_sqr"),
                );
                ui.add(egui::Slider::new(&mut state.outer_size, 1..=19).text("outer_size"));
                ui.add(
                    egui::Slider::new(
                        &mut state.outer_radius_sqr,
                        outer_radius_bounds.0..=outer_radius_bounds.1,
                    )
                    .text("outer_radius_sqr"),
                );
            });

        // store remaining space for macroquad drawing
        editor.canvas = Some(egui_ctx.available_rect());
        editor.egui_wants_mouse = Some(egui_ctx.wants_pointer_input());
    });
}

#[derive(Debug)]
struct State {
    inner_radius_sqr: usize,
    inner_size: usize,
    outer_radius_sqr: usize,
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
    let radius_sqr = walker.kernel.radius_sqr;

    // very crappy hotfix to deal with different center whether size is even or not
    let offset = match size % 2 == 0 {
        true => 0.0,
        false => 0.5,
    };

    draw_circle_lines(
        (walker.pos.x) as f32 + offset,
        (walker.pos.y) as f32 + offset,
        (radius_sqr as f32).sqrt(),
        0.05,
        match flag {
            true => GREEN,
            false => RED,
        },
    );

    draw_circle_lines(
        (walker.pos.x) as f32 + offset,
        (walker.pos.y) as f32 + offset,
        radius_sqr as f32,
        0.025,
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

    let kernel = Kernel::new(3, 1);
    let mut walker = CuteWalker::new(Position::new(10, 10), vec![Position::new(15, 15)], kernel);
    let mut fps_ctrl = FPSControl::new().with_max_fps(60);

    let mut state = State {
        inner_radius_sqr: Kernel::get_valid_radius_bounds(3).0,
        inner_size: 3,
        outer_radius_sqr: Kernel::get_valid_radius_bounds(5).1,
        outer_size: 5,
    };

    Kernel::evaluate_kernels(19);

    loop {
        fps_ctrl.on_frame_start();
        editor.on_frame_start();

        define_egui(&mut editor, &mut state);

        editor.set_cam(&map);
        editor.handle_user_inputs(&map);

        clear_background(GRAY);
        draw_walker(&walker);

        walker.kernel = Kernel::new(state.outer_size, state.outer_radius_sqr);
        draw_thingy(&walker, false);

        let weird_factor: f64 = 2.0 * SQRT_2 * state.outer_radius_sqr.to_f64().sqrt();
        let max_inner_radius_sqr: f64 = state.outer_radius_sqr.to_f64() - weird_factor + 2.0;
        // as this is based on a less or equal equation we can round down to the next integer,
        let valid_inner_radius_sqr: usize = (max_inner_radius_sqr).round().to_usize().unwrap();

        // NOTE: it seems to work with a crappy fix like this using +0.2 ... this is the case
        // because an extra distance of sqrt(2) is required in the "worst case" if the
        // "most outward" blocks are exactly on the limiting radius (e.g. max radius). Then, The full
        // sqrt(2) are required, in other cases less is okay. Therefore the sqrt(2) assumption
        // might not be that useful? What other possible ways could there be to validate if outer
        // kernel has at least "one padding" around the inner kernel?

        // TODO: idea: dont do radius+sqrt(2), but radius-unused+sqrt(2), where unused is the
        // amount of the radius that is not required for active blocks. This means i need to
        // somehow get the "actual limiting radius"

        // NOTE: okay jesus christ im  going insane. it turns out that this entire approach is
        // faulty to begin with. When only using kernel-radii that are exactly limiting the most
        // outter blocks, i expected the remaining margin to be sqrt(2) (see get_unique_radii_sqr).
        // But, it turns out that this is not correct. Imagine 3x3 - 4x4. Then yes. but imagine
        // 2x3 - 3x4, then yes the difference vector is (1,1) which has a distance of sqrt(2). But
        // if you draw the whole thing using vectors you'll see that the distance between the radii
        // and the limited blocks is only both equal to sqrt(2) if the vectors for the limiting
        // are overlapping. I tried to think about doing funky calculations using angles, but the
        // inner circle is not known and therefore the angle is also not known.
        // Instead i came up with a completly new Kernel representation, which would make this
        // entire calculation obsolete. Instead of defining some circularity/radius i could define
        // the 'limiting block' (x, y), which would also implicitly define a radius. I also should ensure
        // that x and y are positive. Then, i could simply reduce -1 from both and have the 'one
        // smaller but valid' kernel! It might also make sense to have some x>=y constraint because
        // otherwise (3,2) and (2,3) would result in the same kernel.
        // This idea could also have some problems that im currently not noticing :D
        // (x,y) are actually the offset to the center, so the largest possible value resulting in
        // a square kernel would be (size - center - 1, size - center - 1) and the smallest possible value
        // would be (0, size - center - 1), which should result in fully circular kernel.
        // the only downside i can think of is that, while i can calculate a circularity (0 - 1)
        // based on these informations, i cant generate a kernel based on some circularity in some
        // trivial way. This would be nice when changing the size of the kernel, but wanting that
        // it remains a similar shape.

        if state.inner_radius_sqr as f64 > max_inner_radius_sqr {
            state.inner_radius_sqr = valid_inner_radius_sqr;
        }

        walker.kernel = Kernel::new(state.inner_size, state.inner_radius_sqr);

        // let valid_radii_sqr = Kernel::get_unique_radii_sqr(state.inner_size);

        // dbg!((
        //     &weird_factor,
        //     &max_inner_radius_sqr,
        //     &valid_inner_radius_sqr,
        //     &state,
        //     &walker.kernel,
        //     &valid_radii_sqr
        // ));
        draw_thingy(&walker, true);

        egui_macroquad::draw();
        fps_ctrl.wait_for_next_frame().await;
    }
}

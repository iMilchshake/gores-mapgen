use egui::{epaint::Shadow, Color32, Frame, Label, Margin, Rect};
use macroquad::prelude::*;


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

    dbg!(Rect::NOTHING);
    dbg!(Rect::NAN);
    dbg!(Rect::EVERYTHING);

    loop {
        clear_background(WHITE);

        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::right("right_panel").show(egui_ctx, |ui| {
                ui.label("hello world");
                ui.separator();
            });

            egui::Window::new("yeah").frame(window_frame()).show(egui_ctx, |ui| {
                ui.add(Label::new("this is some UI stuff"));
                ui.button("text").clicked();
            });

            main_rect = egui_ctx.available_rect();
        });

        // set_camera(&Camera2D {
        //     zoom: vec2(1., -screen_width() / screen_height()),
        //     ..Default::default()
        // });
        


        let rect = main_rect.shrink(50.0);

        draw_rectangle(rect.min.x, rect.min.y, rect.width(), rect.height(), GREEN);


        egui_macroquad::draw();
        next_frame().await
    }
}

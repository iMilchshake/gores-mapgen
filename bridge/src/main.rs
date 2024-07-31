mod color_macros;
mod econ;
mod app;

fn main() {
    app::ServerBridge::<512>::run();
}

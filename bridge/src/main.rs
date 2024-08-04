use simple_logger::SimpleLogger;

#[macro_use]
mod color_macros;

mod app;
mod econ;

fn main() {
    SimpleLogger::new().init().unwrap();
    
    app::ServerBridge::run();
}

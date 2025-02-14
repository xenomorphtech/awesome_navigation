mod debug_draw;
mod viewer;

fn main() -> Result<(), eframe::Error> {
    viewer::run()
}

mod debug_draw_b;
mod obj_loader;
mod viewer;

fn main() -> Result<(), eframe::Error> {
    viewer::run()
}

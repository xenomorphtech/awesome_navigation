mod debug_draw_b;
mod viewer;
mod obj_loader;

fn main() -> Result<(), eframe::Error> {
    viewer::run()
}

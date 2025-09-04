mod home;
mod notes;

pub use home::HomeApplet;
pub use notes::NotesApplet;

pub struct DefaultApplet;
impl Renderable for DefaultApplet {}

pub trait Renderable {
    fn init() {}

    fn render(ctx: &egui::Context) {
        egui::Window::new("debug").resizable(true).show(ctx, |ui| {
            ui.label("unimplemented");
            // Show window info
            if let Some(rect) = ctx.memory(|m| m.area_rect("debug")) {
                ui.label(format!("Size: {:.0} x {:.0}", rect.width(), rect.height()));
            }
        });
    }
}

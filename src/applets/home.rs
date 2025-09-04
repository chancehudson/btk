use super::Applet;

#[derive(Default)]
pub struct HomeApplet {}

impl Applet for HomeApplet {
    fn name(&self) -> &str {
        "Home"
    }

    fn render(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer")
            .exact_height(240.0)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let image = egui::Image::new(egui::include_image!("../../assets/btk.jpg"));
                    ui.add(image.max_height(200.0));
                    ui.label("big tech killer, circa 2025");
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("BTK");
            ui.label("A local first productivity suite.");
            ui.label("An encrypted cloud just for you.");
            ui.label("An attack on surveillance culture.");
            ui.label("An offering for entropy.");
            ui.separator();
            ui.label("The righteous prevail or nothing remains.");
        });
    }
}

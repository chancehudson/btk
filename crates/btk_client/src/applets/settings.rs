use super::Applet;
use crate::app::AppState;

#[derive(Default)]
pub struct SettingsApplet {}

impl Applet for SettingsApplet {
    fn name(&self) -> &str {
        "Settings"
    }

    fn render(&mut self, ctx: &egui::Context, state: &AppState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Application settings");
            ui.label(format!("Remote url: {}", state.network_manager.url()));
            if state.network_manager.is_connected() {
                ui.label("Connected!");
            } else {
                ui.label("Not connected!");
            }
            ui.separator();
            if let Some(path) = &state.local_data.local_data_dir {
                ui.label(format!("Local data path: {:?}", path));
            } else {
                ui.label("Local data path: None (in memory only)");
            }
        });
    }
}

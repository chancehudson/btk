use super::Applet;
use crate::app_state::AppState;

#[derive(Default)]
pub struct SettingsApplet {}

impl Applet for SettingsApplet {
    fn name(&self) -> &str {
        "Settings"
    }

    fn render(&mut self, ctx: &egui::Context, state: &AppState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if state.local_data.active_cloud_id.is_none() {
                ui.heading("no active cloud!");
                return;
            }
            let active_cloud_id = state.local_data.active_cloud_id.unwrap();
            let active_cloud = state.local_data.clouds.get(&active_cloud_id);
            if active_cloud.is_none() {
                ui.heading("WARNING: active cloud is specified but unknown!");
                return;
            }
            let active_cloud = active_cloud.unwrap();
            ui.heading("Cloud settings");
            ui.label(format!("Remote url: {}", state.network_manager.url()));
            if state.network_manager.is_connected() {
                ui.label("Connected!");
            } else {
                ui.label("Not connected!");
            }
            ui.separator();
            if let Some(path) = &active_cloud.filepath() {
                ui.label(format!("Local data path: {:?}", path));
            } else {
                ui.label("Local data path: None (in memory only)");
            }
            ui.separator();
            ui.label(&format!("cloud name: {}", active_cloud.metadata.name));
            ui.label(&format!(
                "cloud description: {}",
                active_cloud.metadata.description
            ));
            ui.label(&format!(
                "cloud created at: {}",
                active_cloud.metadata.created_at
            ));
            ui.label(&format!("cloud id: {}", active_cloud.id_hex()));
        });
    }
}

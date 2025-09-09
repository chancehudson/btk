use anyhow::Result;

use crate::app::ActionRequest;
use crate::app::AppEvent;
use crate::app_state::AppState;
use crate::applets::Applet;

#[derive(Default)]
pub struct SettingsApplet {
    new_remote_url: String,
}

impl Applet for SettingsApplet {
    fn name(&self) -> &str {
        "Settings"
    }

    fn handle_app_events(&mut self, events: &Vec<AppEvent>, state: &AppState) -> Result<()> {
        for event in events {
            match event {
                AppEvent::ActiveAppletChanged => {
                    self.new_remote_url = String::default();
                }
                AppEvent::ActiveCloudChanged => {
                    self.new_remote_url = String::default();
                }
            }
        }
        Ok(())
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
            ui.separator();
            ui.label("Remote connection");
            ui.label(&format!(
                "url: {}",
                active_cloud
                    .metadata
                    .remote_url
                    .clone()
                    .unwrap_or("None".to_string())
            ));
            let response = egui::TextEdit::singleline(&mut self.new_remote_url)
                .hint_text("ws://localhost:5001")
                .show(ui)
                .response;
            if response.has_focus() && self.new_remote_url.len() > 3 {
                response.show_tooltip_ui(|ui| {
                    ui.label("Press enter to save");
                });
            }
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                // TODO: first check if we're overwriting an existing note
                let remote_url = if self.new_remote_url.is_empty() {
                    None
                } else {
                    Some(std::mem::take(&mut self.new_remote_url))
                };
                let mut new_metadata = active_cloud.metadata.clone();
                new_metadata.remote_url = remote_url;
                state
                    .pending_requests
                    .0
                    .send(ActionRequest::UpdateCloudMetadata(
                        active_cloud_id,
                        new_metadata,
                    ))
                    .expect("failed to send update cloud metadata action request");
            }
        });
    }
}

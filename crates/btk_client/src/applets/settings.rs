use anyhow::Result;

use crate::app::ActionRequest;
use crate::app::AppEvent;
use crate::app_state::AppState;
use crate::applets::Applet;
use crate::widgets::EditableLabel;

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

            ui.label(&format!("id: {}", active_cloud.id_hex()));
            ui.label(&format!("created at: {}", active_cloud.metadata.created_at));

            ui.horizontal(|ui| {
                ui.label("name:");
                let mut name_label =
                    EditableLabel::init(format!("{}-name", active_cloud.id_hex()), ui, &|label| {});
                if name_label.changed() {
                    let mut new_metadata = active_cloud.metadata.clone();
                    new_metadata.name = name_label.value.clone();
                    state
                        .pending_requests
                        .0
                        .send(ActionRequest::UpdateCloudMetadata(
                            active_cloud_id,
                            new_metadata,
                        ))
                        .expect("failed to send update cloud metadata action request");
                }
                name_label.update_value_if_needed(&active_cloud.metadata.name);
                ui.add(name_label);
            });
            ui.horizontal(|ui| {
                ui.label("description:");
                let mut description_label = EditableLabel::init(
                    format!("{}-description", active_cloud.id_hex()),
                    ui,
                    &|label| {},
                );
                if description_label.changed() {
                    let mut new_metadata = active_cloud.metadata.clone();
                    new_metadata.description = description_label.value.clone();
                    state
                        .pending_requests
                        .0
                        .send(ActionRequest::UpdateCloudMetadata(
                            active_cloud_id,
                            new_metadata,
                        ))
                        .expect("failed to send update cloud metadata action request");
                }
                description_label.update_value_if_needed(&active_cloud.metadata.description);
                ui.add(description_label);
            });

            ui.separator();
            ui.label("Remote connection");
            ui.horizontal(|ui| {
                ui.label("remote url");
                let mut url_label =
                    EditableLabel::init(format!("{}-url", active_cloud.id_hex()), ui, &|label| {});
                if url_label.changed() {
                    let mut new_metadata = active_cloud.metadata.clone();
                    if url_label.value.trim().is_empty() {
                        new_metadata.remote_url = None;
                    } else {
                        new_metadata.remote_url = Some(url_label.value.to_string());
                    }
                    state
                        .pending_requests
                        .0
                        .send(ActionRequest::UpdateCloudMetadata(
                            active_cloud_id,
                            new_metadata,
                        ))
                        .expect("failed to send update cloud metadata action request");
                }
                if let Some(remote_url) = &active_cloud.metadata.remote_url {
                    url_label.update_value_if_needed(&remote_url);
                } else {
                    url_label.update_value_if_needed("");
                }
                ui.add(url_label);
            });
        });
    }
}

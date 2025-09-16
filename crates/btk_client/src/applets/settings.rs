use anyhow::Result;
use egui::Color32;

use crate::app::ActionRequest;
use crate::app::AppEvent;
use crate::applets::Applet;
use crate::data::AppState;
use crate::widgets::EditableLabel;

#[derive(Default)]
pub struct SettingsApplet {
    new_remote_url: String,
}

impl Applet for SettingsApplet {
    fn name(&self) -> &str {
        "Settings"
    }

    fn handle_app_events(&mut self, events: &Vec<AppEvent>, _state: &AppState) -> Result<()> {
        for event in events {
            match event {
                AppEvent::ActiveAppletChanged(_applet_name) => {
                    self.new_remote_url = String::default();
                }
                AppEvent::ActiveCloudChanged(_applet_name) => {
                    self.new_remote_url = String::default();
                }
                AppEvent::RemoteCloudUpdate(_cloud_id) => {
                    // nothing to handle
                }
            }
        }
        Ok(())
    }

    fn render(&mut self, ctx: &egui::Context, state: &AppState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if state.active_cloud_id.is_none() {
                ui.heading("no active cloud!");
                return;
            }
            let active_cloud_id = state.active_cloud_id.unwrap();
            let (active_cloud, metadata) = match state.cloud_by_id(&active_cloud_id) {
                Some(v) => v,
                None => {
                    ui.heading("WARNING: active cloud is specified but unknown!");
                    return;
                }
            };
            ui.heading("Cloud settings");
            ui.separator();
            if let Some(path) = &active_cloud.filepath() {
                ui.label(format!("Local data path: {:?}", path));
            } else {
                ui.label("Local data path: None (in memory only)");
            }
            ui.separator();

            ui.label(&format!("id: {}", active_cloud.id_hex()));
            ui.label(&format!("created at: {}", metadata.created_at));

            ui.horizontal(|ui| {
                ui.label("name:");
                let mut name_label = EditableLabel::init(
                    format!("{}-name", active_cloud.id_hex()),
                    ui,
                    &|_label| {},
                );
                if name_label.changed() {
                    let mut new_metadata = metadata.clone();
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
                name_label.update_value_if_needed(&metadata.name);
                ui.add(name_label);
            });
            ui.horizontal(|ui| {
                ui.label("description:");
                let mut description_label = EditableLabel::init(
                    format!("{}-description", active_cloud.id_hex()),
                    ui,
                    &|_label| {},
                );
                if description_label.changed() {
                    let mut new_metadata = metadata.clone();
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
                description_label.update_value_if_needed(&metadata.description);
                ui.add(description_label);
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("key:");
                ui.label(hex::encode(active_cloud.private_key()));
            });
            ui.colored_label(
                Color32::RED,
                "WARNING: sharing this key irreversibly shares access to this cloud!",
            );

            ui.separator();
            ui.label("Remote connection");
            let remote = state
                .remote_clouds
                .read()
                .unwrap()
                .get(&active_cloud_id)
                .cloned();
            if remote.is_none() {
                return;
            }
            let remote = remote.unwrap();
            ui.horizontal(|ui| {
                ui.label("http url:");
                ui.label(remote.http_url());
            });
            ui.horizontal(|ui| {
                ui.label("ws url:");
                ui.label(remote.ws_url());
            });
            ui.horizontal(|ui| {
                ui.label("confirmed mutations:");
                ui.label(format!(
                    "{}",
                    remote
                        .latest_confirmed_index()
                        .and_then(|v| Some((v + 1).to_string()))
                        .or_else(|| Some("None".to_string()))
                        .unwrap()
                ));
            });
            ui.horizontal(|ui| {
                ui.label("synchronization:");
                if remote.synchronization_enabled() {
                    ui.colored_label(Color32::GREEN, "enabled");
                    if ui.button("disable").clicked() {
                        remote.set_synchronization_enabled(false).ok();
                    }
                } else {
                    ui.colored_label(Color32::RED, "disabled");
                    if ui.button("enable").clicked() {
                        remote.set_synchronization_enabled(true).ok();
                    }
                }
            });
        });
    }
}

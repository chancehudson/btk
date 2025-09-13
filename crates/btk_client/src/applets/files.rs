use std::fs;
use std::sync::Arc;

use anondb::Bytes;
use anyhow::Result;
use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::Overflow;
use egui_taffy::taffy::Point;
use egui_taffy::taffy::prelude::*;

use super::Applet;
use crate::app::AppEvent;
use crate::data::AppState;

#[derive(Default)]
pub struct FilesApplet {
    filenames: Vec<String>,
    showing_add_file_window: bool,
    add_file_name: String,
    add_file_bytes: Arc<[u8]>,
}

impl FilesApplet {
    fn load_files(&mut self, state: &AppState) -> Result<()> {
        let active_cloud = state.active_cloud();
        if active_cloud.is_none() {
            println!("WARNING: trying to load files with no active cloud");
        }
        let (cloud, _metadata) = active_cloud.unwrap();
        self.filenames = cloud.db.list_keys::<String>("files")?;
        Ok(())
    }

    fn render_add_file_window(&mut self, ctx: &egui::Context, state: &AppState) {
        let viewport_size = ctx.screen_rect().size();
        let window_size = egui::Vec2::new(300.0, 300.0);
        egui::Window::new("add file")
            .default_size(window_size)
            .default_pos([
                (viewport_size.x - window_size.x) * 0.5,
                (viewport_size.y - window_size.y) * 0.5,
            ])
            .collapsible(false)
            .show(ctx, |ui| {
                let text_edit = egui::TextEdit::singleline(&mut self.add_file_name)
                    .hint_text("filename")
                    .desired_width(window_size.x);
                let input = ui.add(text_edit);

                input.request_focus();
                ui.vertical_centered(|ui| {
                    if ui.button("save").clicked() {
                        if let Some((cloud, _)) = state.active_cloud() {
                            cloud
                                .db
                                .insert::<String, Bytes>(
                                    "files",
                                    &self.add_file_name,
                                    &std::mem::take(&mut self.add_file_bytes).into(),
                                )
                                .expect("failed to add file");
                            self.load_files(state).ok();
                            self.showing_add_file_window = false;
                            self.add_file_name = String::default();
                        } else {
                            println!("WARNING: no active cloud!");
                        }
                    }
                    if ui.button("cancel").clicked() {
                        self.showing_add_file_window = false;
                        std::mem::take(&mut self.add_file_bytes);
                        self.add_file_name = String::default();
                    }
                });
            });
    }
}

impl Applet for FilesApplet {
    fn name(&self) -> &str {
        "Files"
    }

    fn handle_app_events(
        &mut self,
        events: &Vec<crate::app::AppEvent>,
        state: &AppState,
    ) -> Result<()> {
        for event in events {
            match event {
                AppEvent::ActiveCloudChanged => {
                    self.load_files(state)?;
                }
                AppEvent::RemoteCloudUpdate(cloud_id) => {
                    if cloud_id == &state.active_cloud_id.unwrap_or_default() {
                        self.load_files(state)?;
                    }
                }
                AppEvent::ActiveAppletChanged => {
                    // self.load_files(state)?;
                }
            }
        }
        Ok(())
    }

    fn render(&mut self, ctx: &egui::Context, state: &AppState) {
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if i.raw.dropped_files.len() > 1 {
                    println!("WARNING: may only drop 1 file at a time");
                    return;
                }
                for file in &i.raw.dropped_files {
                    if let Some(bytes) = file.bytes.as_ref() {
                        self.add_file_name = file.name.to_string();
                        self.add_file_bytes = bytes.clone();
                        self.showing_add_file_window = true;
                    } else if let Some(path) = file.path.as_ref() {
                        let name = path.file_name().and_then(|name| name.to_str());
                        if name.is_none() {
                            println!("WARNING: unable to extract filename");
                            return;
                        }
                        let name = name.unwrap().to_string();
                        match fs::read(path) {
                            Ok(bytes) => {
                                self.add_file_bytes = bytes.into();
                                self.add_file_name = name;
                                self.showing_add_file_window = true;
                            }
                            Err(e) => {
                                println!("failed to read file! {:?}", e);
                                return;
                            }
                        }
                    } else {
                        println!("no file info!");
                        return;
                    }
                }
            }
        });

        if self.showing_add_file_window {
            self.render_add_file_window(ctx, state);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Files");
            });
            egui_taffy::tui(ui, "home")
                .reserve_available_space()
                .style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    min_size: Size {
                        width: percent(1.0),
                        height: length(0.0),
                    },
                    max_size: Size {
                        width: percent(1.0),
                        height: percent(1.0),
                    },
                    overflow: Point {
                        x: Overflow::default(),
                        y: Overflow::Scroll,
                    },
                    ..Default::default()
                })
                .show(|tui| {
                    for name in &self.filenames {
                        tui.heading(name);
                    }
                });
        });
    }
}

use std::fs;
use std::sync::Arc;

use anondb::Bytes;
use anyhow::Result;
use egui_commonmark::CommonMarkCache;
use egui_commonmark::CommonMarkViewer;
use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::Overflow;
use egui_taffy::taffy::Point;
use egui_taffy::taffy::prelude::*;

use super::Applet;
use crate::app::AppEvent;
use crate::data::AppState;
use crate::widgets::ConfirmButton;

#[derive(Default)]
pub struct FilesApplet {
    filenames: Vec<String>,
    showing_add_file_window: bool,
    add_file_name: String,
    add_file_bytes: Arc<[u8]>,
    selected_filename: String,
    selected_file_bytes: Vec<u8>,
}

impl FilesApplet {
    #[cfg(target_arch = "wasm32")]
    fn download_selected_file(&mut self) -> Result<()> {
        let blob = gloo_file::Blob::new_with_options(self.selected_file_bytes.as_slice(), None);

        // Create download link
        let url = web_sys::Url::create_object_url_with_blob(&blob.into()).unwrap();

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let anchor = document.create_element("a").unwrap();
        anchor.set_attribute("href", &url).unwrap();
        anchor
            .set_attribute("download", &self.selected_filename)
            .unwrap();

        // Trigger click
        use wasm_bindgen_futures::wasm_bindgen::JsCast;
        anchor.unchecked_into::<web_sys::HtmlElement>().click();

        web_sys::Url::revoke_object_url(&url).ok();
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn download_selected_file(&mut self) -> Result<()> {
        let dir = tempfile::tempdir()?;
        let temp_file = dir.path().join(&self.selected_filename);

        fs::write(&temp_file, &self.selected_file_bytes)?;

        open::that(dir.path()).ok();

        _ = dir.keep();
        Ok(())
    }

    fn delete_selected_file(&mut self, state: &AppState) -> Result<()> {
        if let Some((active_cloud, _)) = state.active_cloud() {
            active_cloud
                .db
                .remove::<String, Bytes>("files", &self.selected_filename)?;
            self.selected_filename = String::default();
            self.selected_file_bytes = Vec::default();
            self.load_files(state)?;
        }
        Ok(())
    }

    fn load_selected_file(&mut self, state: &AppState) {
        if let Some((cloud, _)) = state.active_cloud() {
            self.selected_file_bytes = cloud
                .db
                .get::<String, Bytes>("files", &self.selected_filename)
                .unwrap_or_else(|e| {
                    println!("WARNING: failed to load selected file: {e:?}");
                    None
                })
                .unwrap_or_default()
                .to_vec();
        } else {
            println!("WARNING: no active cloud!");
        }
    }

    fn load_files(&mut self, state: &AppState) -> Result<()> {
        let active_cloud = state.active_cloud();
        if active_cloud.is_none() {
            println!("WARNING: trying to load files with no active cloud");
            return Ok(());
        }
        let (cloud, _metadata) = active_cloud.unwrap();
        self.filenames = cloud.db.list_keys::<String>("files")?;
        Ok(())
    }

    fn render_add_file_window(&mut self, ctx: &egui::Context, state: &AppState) {
        let window_size = egui::Vec2::new(300.0, 300.0);
        let response = egui::Modal::new("add file".into()).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Add file to cloud");
            });
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("name:");
                let text_edit = egui::TextEdit::singleline(&mut self.add_file_name)
                    .hint_text("filename")
                    .desired_width(window_size.x);
                let input = ui.add(text_edit);
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.showing_add_file_window = false;
                    self.add_file_name = String::default();
                }

                if input.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
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
                input.request_focus();
            });
            ui.add_space(4.0);
            ui.vertical_centered(|ui| {
                if ui.button("cancel").clicked() {
                    self.showing_add_file_window = false;
                    std::mem::take(&mut self.add_file_bytes);
                    self.add_file_name = String::default();
                }
            });
        });
        if response.should_close() {
            self.showing_add_file_window = false;
            self.add_file_name = String::default();
        }
    }

    fn render_file_info(&mut self, ctx: &egui::Context, state: &AppState) {
        let viewport_size = ctx.screen_rect();
        egui::SidePanel::right("file_info")
            .default_width((viewport_size.width() / 2.0).min(500.0))
            .show(ctx, |ui| {
                egui_taffy::tui(ui, "file_info_inner")
                    .reserve_available_space()
                    .style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        min_size: Size {
                            width: percent(1.0),
                            height: percent(1.0),
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
                        tui.style(Style {
                            flex_direction: FlexDirection::Row,
                            justify_content: Some(JustifyContent::SpaceBetween),
                            align_items: Some(AlignItems::FlexEnd),
                            ..Default::default()
                        })
                        .add(|tui| {
                            tui.heading(&self.selected_filename);
                            tui.ui(|ui| {
                                let delete_button =
                                    ConfirmButton::init("file_info_delete".to_string(), ui, &|b| {
                                        b.text = "Delete".to_string();
                                        b.confirm_text = "Are you sure?".to_string();
                                    });
                                if delete_button.confirmed() {
                                    self.delete_selected_file(state).ok();
                                }
                                ui.add(delete_button);
                            });
                        });
                        tui.ui(|ui| ui.add_space(4.0));
                        tui.separator();
                        tui.ui(|ui| ui.add_space(4.0));
                        let file_extension = self
                            .selected_filename
                            .split(".")
                            .last()
                            .unwrap_or_default()
                            .to_string();
                        tui.label(format!("{} bytes", self.selected_file_bytes.len()));
                        tui.ui(|ui| ui.add_space(4.0));
                        if tui
                            .style(Style {
                                padding: length(4.0),
                                ..Default::default()
                            })
                            .button(|tui| {
                                tui.label("Download");
                            })
                            .clicked()
                        {
                            self.download_selected_file().ok();
                        }
                        tui.ui(|ui| ui.add_space(4.0));
                        let is_image = file_extension == "jpg"
                            || file_extension == "jpeg"
                            || file_extension == "png"
                            || file_extension == "gif"
                            || file_extension == "webp";
                        #[cfg(not(target_arch = "wasm32"))]
                        if is_image {
                            if tui
                                .style(Style {
                                    padding: length(4.0),
                                    ..Default::default()
                                })
                                .button(|tui| {
                                    tui.label("Copy to clipboard");
                                })
                                .clicked()
                            {
                                if let Some(mut clipboard) = arboard::Clipboard::new().ok()
                                    && let Some(image) = ::image::load_from_memory(
                                        self.selected_file_bytes.as_slice(),
                                    )
                                    .ok()
                                {
                                    clipboard
                                        .set_image(arboard::ImageData {
                                            width: image.width() as usize,
                                            height: image.height() as usize,
                                            bytes: image.to_rgba8().to_vec().into(),
                                        })
                                        .ok();
                                }
                            }
                        }
                        tui.ui(|ui| ui.add_space(4.0));
                        tui.separator();
                        tui.ui(|ui| ui.add_space(4.0));
                        if is_image {
                            let image = egui::Image::from_bytes(
                                self.selected_filename.clone(),
                                self.selected_file_bytes.clone(),
                            );
                            tui.style(Style {
                                size: Size {
                                    height: percent(1.0),
                                    width: percent(1.0),
                                },
                                padding: length(4.0),
                                ..Default::default()
                            })
                            .ui_add(image);
                        } else if file_extension == "md" {
                            tui.ui(|ui| {
                                ui.style_mut().wrap_mode = Default::default();
                                ui.set_max_width(ui.available_width());
                                // TODO: actually cache this :(
                                let string = String::from_utf8(self.selected_file_bytes.clone())
                                    .unwrap_or_default();
                                let mut md_cache = CommonMarkCache::default();
                                CommonMarkViewer::new().show(ui, &mut md_cache, &string);
                            });
                        } else {
                            tui.label("Cannot preview filetype");
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
                    self.selected_filename = String::default();
                }
                AppEvent::RemoteCloudUpdate(cloud_id) => {
                    if cloud_id == &state.active_cloud_id.unwrap_or_default() {
                        self.load_files(state)?;
                    }
                }
                AppEvent::ActiveAppletChanged(applet_name) => {
                    if applet_name == self.name() {
                        self.load_files(state)?;
                        self.selected_filename = String::default();
                    }
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

        if !self.selected_filename.trim().is_empty() {
            self.render_file_info(ctx, state);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Files");
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
                    let mut selected_file_changed = false;
                    for name in &self.filenames {
                        if tui
                            .style(Style {
                                flex_direction: FlexDirection::Row,
                                justify_content: Some(JustifyContent::SpaceBetween),
                                padding: length(4.0),
                                margin: length(2.0),
                                ..Default::default()
                            })
                            .selectable(&self.selected_filename == name, |tui| {
                                tui.heading(name);
                            })
                            .clicked()
                        {
                            self.selected_filename = name.clone();
                            selected_file_changed = true;
                        }
                    }
                    if selected_file_changed {
                        self.load_selected_file(state);
                    }
                });
        });
    }
}

use anyhow::Result;
use egui::Rect;
use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::Overflow;
use egui_taffy::taffy::Point;
use egui_taffy::taffy::prelude::*;
use indexmap::IndexMap;
use web_time::Duration;
use web_time::Instant;

use crate::applets::*;
use crate::data::AppState;
use crate::data::CloudMetadata;

pub enum AppEvent {
    ActiveAppletChanged,
    ActiveCloudChanged,
    /// An update was received from the remote. The ui should be resynced
    RemoteCloudUpdate([u8; 32]),
}

pub enum ActionRequest {
    LoadClouds,
    // the id to switch to
    SwitchCloud([u8; 32]),
    UpdateCloudMetadata([u8; 32], CloudMetadata),
}

pub struct App {
    state: AppState,
    show_stats: bool,
    show_clouds_menu: bool,
    last_render_time: Duration,
    active_applet: String,
    applets: IndexMap<String, Box<dyn Applet>>,
    showing_import: bool,
    import_key: String,
}

#[cfg(target_arch = "wasm32")]
fn webapp_href() -> Result<Option<String>> {
    if let Some(window) = web_sys::window() {
        let location = window.location();
        Ok(location.href().ok())
    } else {
        Ok(None)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn webapp_href() -> Result<Option<String>> {
    Ok(None)
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self> {
        // setup egui/taffy rendering stuff
        egui_extras::install_image_loaders(&cc.egui_ctx);

        cc.egui_ctx.options_mut(|options| {
            options.max_passes = std::num::NonZeroUsize::new(2).unwrap();
        });
        cc.egui_ctx.style_mut(|style| {
            style.wrap_mode = Some(egui::TextWrapMode::Extend);
        });

        // construct application state
        let mut state = AppState::new()?;

        state.init()?;

        if let Some(cloud_id) = state.active_cloud_id {
            // trigger an event being sent so applets can handle loading
            state.switch_cloud(cloud_id);
        }

        let mut applets: IndexMap<String, _> = IndexMap::new();

        for mut applet in vec![
            Box::new(FilesApplet::default()) as Box<dyn Applet>,
            Box::new(NotesApplet::default()) as Box<dyn Applet>,
            Box::new(TasksApplet::default()) as Box<dyn Applet>,
            Box::new(MailApplet::default()) as Box<dyn Applet>,
            Box::new(SettingsApplet::default()) as Box<dyn Applet>,
        ] {
            applet.as_mut().init(&state)?;
            applets.insert(applet.name().into(), applet);
        }

        let mut out = Self {
            state,
            active_applet: applets
                .first()
                .expect("no applets registered; line break pls")
                .0
                .into(),
            applets,
            show_stats: cfg!(debug_assertions),
            last_render_time: Duration::default(),
            show_clouds_menu: false,
            showing_import: false,
            import_key: String::default(),
        };

        // on the web allow customizing the initial view
        if let Some(href) = webapp_href()? {
            let url = reqwest::Url::parse(&href)?;
            for (key, val) in url.query_pairs() {
                if key == "clouds" {
                    out.show_clouds_menu = true;
                }
                if key == "import" {
                    out.showing_import = true;
                }
            }
        }
        Ok(out)
    }
}

impl App {
    fn change_applet(&mut self, next_applet: String) {
        self.active_applet = next_applet;
        self.state
            .pending_events
            .0
            .send(AppEvent::ActiveAppletChanged)
            .expect("failed to send app event");
    }

    fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        // Use CMD+num_key to switch to an applet
        let number_keys = [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
            egui::Key::Num7,
            egui::Key::Num8,
            egui::Key::Num9,
        ];
        for (index, &key) in number_keys.iter().enumerate() {
            if index >= self.applets.iter().len() {
                break;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, key)) {
                // attempt to switch to the relevant appley
                let (applet_name, _) = self
                    .applets
                    .get_index(index)
                    .expect("logical mismatch between applets len and index");
                self.change_applet(applet_name.clone());
            }
        }
    }

    fn show_framerate_window(&self, ctx: &egui::Context) {
        // frame render time stats
        if self.show_stats {
            egui::Window::new("Frame Stats")
                .default_rect(Rect::from_min_size(
                    [ctx.screen_rect().size().x - 200.0, 50.0].into(),
                    [150.0, 100.0].into(),
                ))
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    let frame_time_ms = self.last_render_time.as_secs_f32() * 1000.0;
                    let fps = 1.0 / self.last_render_time.as_secs_f32();

                    ui.label(format!("Render time: {:.2} ms", frame_time_ms));
                    ui.label(format!("FPS: {:.1}", fps));
                });
        }
    }

    fn render_clouds_menu(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("clouds_side_menu")
            .resizable(true)
            .default_width(200.0)
            .width_range(150.0..=500.0)
            .show(ctx, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Clouds");
                        if ui.button("+").clicked() {
                            self.state.create_cloud().expect("failed to create cloud");
                            self.state.load_clouds().expect("failed to load clouds");
                        }
                        if ui.button("import").clicked() {
                            self.showing_import = true;
                            self.import_key = String::default();
                        }
                    });
                });

                ui.separator();

                egui_taffy::tui(ui, "cloud_list")
                    .reserve_available_space()
                    .style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        padding: egui_taffy::taffy::Rect {
                            right: length(20.0), // don't overlap the scroll bar
                            left: length(0.0),
                            bottom: length(0.0),
                            top: length(0.0),
                        },
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
                        for (cloud, metadata) in &self.state.sorted_clouds {
                            tui.style(Style {
                                flex_direction: FlexDirection::Row,
                                align_items: Some(AlignItems::Center),
                                justify_content: Some(JustifyContent::SpaceBetween),
                                gap: length(8.0),
                                ..Default::default()
                            })
                            .add(|tui| {
                                if tui
                                    .style(Style {
                                        padding: length(4.0),
                                        margin: length(2.0),
                                        ..Default::default()
                                    })
                                    .selectable(
                                        *cloud.id()
                                            == self.state.active_cloud_id.unwrap_or_default(),
                                        |tui| {
                                            tui.heading(&metadata.name);
                                        },
                                    )
                                    .clicked()
                                {
                                    self.state.switch_cloud(*cloud.id());
                                }
                            });
                        }
                    });
            });
    }

    fn render_import_view(&mut self, ctx: &egui::Context) {
        egui::Window::new("import cloud").show(ctx, |ui| {
            ui.label("enter your private key");
            let input = ui.text_edit_singleline(&mut self.import_key);

            if self.import_key.len() == 64 {
                input.show_tooltip_ui(|ui| {
                    ui.label("press enter to import");
                });
            }

            if input.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                match self.state.import_cloud(&self.import_key) {
                    Ok(cloud_id) => {
                        self.state.load_clouds().unwrap();
                        self.state.set_active_cloud(cloud_id).unwrap();
                        self.showing_import = false;
                        self.import_key = String::default();
                    }
                    Err(_) => {}
                }
            }

            input.request_focus();
            if ui.button("cancel").clicked() {
                self.showing_import = false;
                self.import_key = String::default();
            }
        });
    }

    fn render_footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("root_footer").show(ctx, |ui| {
            egui_taffy::tui(ui, "root_footer_taffy")
                .reserve_available_space()
                .style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: Some(JustifyContent::SpaceBetween),
                    min_size: Size {
                        width: percent(1.0),
                        height: length(0.0),
                    },
                    max_size: Size {
                        width: percent(1.0),
                        height: percent(1.0),
                    },
                    ..Default::default()
                })
                .show(|tui| {
                    if let Some((_cloud, metadata)) = self.state.active_cloud() {
                        tui.label(&metadata.name);
                    } else {
                        tui.label("No active cloud!");
                    }
                    tui.label("synchronizing...");
                });
        });
    }
}

impl eframe::App for App {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_secs(1));
        let render_start = Instant::now();

        let pending_events = self.state.pending_events.1.drain().collect();
        for applet in self.applets.values_mut() {
            applet
                .handle_app_events(&pending_events, &self.state)
                .expect(&format!("applet {} failed to handle events", applet.name()));
        }
        for event in &pending_events {
            if matches!(event, AppEvent::RemoteCloudUpdate(_)) {
                self.state.reload_clouds();
                break;
            }
        }
        // we resend here so the `update` function in the active applet can access these. The
        // channel will be cleared at the end of this function regardless.
        for event in pending_events {
            self.state
                .pending_events
                .0
                .send(event)
                .expect("failed to resend event");
        }

        self.handle_keyboard_input(ctx);
        self.show_framerate_window(ctx);
        self.render_footer(ctx);
        if self.showing_import {
            self.render_import_view(ctx);
        }

        // top tab bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.columns(2, |columns| {
                columns[0].horizontal(|ui| {
                    let last_value = self.show_clouds_menu;
                    if ui
                        .selectable_value(&mut self.show_clouds_menu, true, "☁")
                        .clicked()
                    {
                        if last_value && self.show_clouds_menu {
                            self.show_clouds_menu = false;
                        }
                    }
                    let mut next_applet_maybe: Option<String> = None;
                    for name in self.applets.keys() {
                        if ui
                            .selectable_value(&mut self.active_applet, name.to_string(), name)
                            .changed()
                        {
                            next_applet_maybe = Some(self.active_applet.clone());
                        };
                    }
                    if let Some(next_applet) = next_applet_maybe {
                        self.change_applet(next_applet);
                    }
                });
                columns[1].with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.toggle_value(&mut self.show_stats, "Stats");
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::widgets::global_theme_preference_buttons(ui);
                    });
                })
            });
        });

        if self.show_clouds_menu {
            self.render_clouds_menu(ctx);
        }

        // applet content renderer
        if let Some(applet) = self.applets.get_mut(&self.active_applet) {
            applet.render(ctx, &self.state);
        } else {
            egui::Window::new("unknown applet")
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label("unknown applet selected: ");
                    ui.label(self.active_applet.to_string());
                });
        }
        self.last_render_time = Instant::now().duration_since(render_start);
        self.state.pending_events.1.drain();
        for r in self.state.drain_pending_app_requests() {
            match r {
                ActionRequest::UpdateCloudMetadata(cloud_id, new_metadata) => {
                    if let Some((cloud, _)) = self.state.cloud_by_id(&cloud_id) {
                        cloud
                            .set_metadata(new_metadata)
                            .expect("failed to update cloud metadata");
                        self.state
                            .load_clouds()
                            .expect("failed to load clouds after metadata update");
                    } else {
                        println!("WARNING: attempting to update metadata for unknown cloud");
                    }
                }
                ActionRequest::LoadClouds => {
                    self.state.load_clouds().expect("failed to load clouds");
                }
                ActionRequest::SwitchCloud(cloud_id) => {
                    self.state
                        .set_active_cloud(cloud_id)
                        .expect("failed to set active cloud");
                    self.state
                        .pending_events
                        .0
                        .send(AppEvent::ActiveCloudChanged)
                        .expect("failed to send ActiveCloudChanged app event");
                }
            }
        }
    }
}

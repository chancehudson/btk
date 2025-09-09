use anyhow::Result;
use egui::Rect;
use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::Overflow;
use egui_taffy::taffy::Point;
use egui_taffy::taffy::prelude::*;
use indexmap::IndexMap;
use web_time::Duration;
use web_time::Instant;

use crate::app_state::AppState;
use crate::applets::*;
use crate::data::CloudMetadata;
use crate::data::LocalState;

pub enum AppEvent {
    ActiveAppletChanged,
    ActiveCloudChanged,
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
        let mut state = AppState {
            local_data: LocalState::new()?,
            pending_events: flume::unbounded(),
            pending_requests: flume::unbounded(),
        };

        state.local_data.init()?;
        if let Some(cloud_id) = state.local_data.active_cloud_id {
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

        Ok(Self {
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
        })
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
        egui::SidePanel::left("clouds_menu")
            .resizable(true)
            .default_width(200.0)
            .width_range(150.0..=500.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Clouds");
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
                        for cloud in &self.state.local_data.sorted_clouds {
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
                                            == self
                                                .state
                                                .local_data
                                                .active_cloud_id
                                                .unwrap_or_default(),
                                        |tui| {
                                            tui.heading(&cloud.metadata.name);
                                        },
                                    )
                                    .clicked()
                                {
                                    self.state.switch_cloud(*cloud.id());
                                }
                                tui.ui(|ui| {
                                    if ui.button("⚙").clicked() {
                                        // Settings action
                                    }
                                });
                            });
                        }
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
        for cloud in self.state.local_data.clouds.values_mut() {
            cloud
                .update()
                .expect(&format!("cloud {} failed to update", cloud.metadata.name));
        }

        let render_start = Instant::now();

        let pending_events = self.state.pending_events.1.drain().collect();
        for applet in self.applets.values_mut() {
            applet
                .handle_app_events(&pending_events, &self.state)
                .expect(&format!("applet {} failed to handle events", applet.name()));
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
                    if let Some(cloud) = self.state.local_data.clouds.get_mut(&cloud_id) {
                        cloud
                            .set_metadata(new_metadata)
                            .expect("failed to update cloud metadat");
                    } else {
                        println!("WARNING: attempting to update metadata for unknown cloud");
                    }
                }
                ActionRequest::LoadClouds => {
                    self.state
                        .local_data
                        .load_clouds()
                        .expect("failed to load clouds");
                }
                ActionRequest::SwitchCloud(cloud_id) => {
                    self.state
                        .local_data
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

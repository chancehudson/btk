use anyhow::Result;
use egui::Rect;
use indexmap::IndexMap;
use web_time::Duration;
use web_time::Instant;

use crate::applets::*;
use crate::data::LocalState;
use crate::network::DEFAULT_SERVER_URL;
use crate::network::NetworkManager;

pub struct AppState {
    pub network_manager: NetworkManager,
    pub local_data: LocalState,
}

pub struct App {
    state: AppState,
    show_stats: bool,
    last_render_time: Duration,
    active_applet: String,
    applets: IndexMap<String, Box<dyn Applet>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let state = AppState {
            network_manager: NetworkManager::new(DEFAULT_SERVER_URL),
            local_data: LocalState::new().unwrap(),
        };

        let mut applets: IndexMap<String, _> = IndexMap::new();

        for mut applet in vec![
            Box::new(HomeApplet::default()) as Box<dyn Applet>,
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
            #[cfg(debug_assertions)]
            show_stats: true,
            last_render_time: Duration::default(),
        })
    }
}

impl App {
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
                self.active_applet = applet_name.into();
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
}

impl eframe::App for App {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // egui_taffy stuff
        // Enable multipass rendering upon request without drawing to screen
        ctx.options_mut(|options| {
            options.max_passes = std::num::NonZeroUsize::new(3).unwrap();
        });

        // Disable text wrapping
        //
        // egui text layouting tries to utilize minimal width possible
        ctx.style_mut(|style| {
            style.wrap_mode = Some(egui::TextWrapMode::Extend);
        });

        let render_start = Instant::now();

        self.handle_keyboard_input(ctx);
        self.show_framerate_window(ctx);

        if let Ok(msgs) = self.state.network_manager.receive() {
            if !msgs.is_empty() {
                println!("{} message received", msgs.len());
            }
        }

        // top tab bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.columns(2, |columns| {
                columns[0].horizontal(|ui| {
                    for name in self.applets.keys() {
                        ui.selectable_value(&mut self.active_applet, name.to_string(), name);
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
    }
}

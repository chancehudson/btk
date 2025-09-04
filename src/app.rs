use super::applets;
use super::applets::Renderable;

#[derive(Default)]
pub struct App {
    active_applet: Applet,
}

#[derive(Debug, Default, PartialEq)]
enum Applet {
    #[default]
    Home,
    Notes,
    TodoList,
    Settings,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        applets::HomeApplet::init();
        applets::NotesApplet::init();

        Default::default()
    }
}

impl eframe::App for App {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.columns(2, |columns| {
                columns[0].horizontal(|ui| {
                    ui.selectable_value(&mut self.active_applet, Applet::Home, "Home");
                    ui.selectable_value(&mut self.active_applet, Applet::Notes, "Notes");
                    ui.selectable_value(&mut self.active_applet, Applet::TodoList, "Todo");
                    ui.selectable_value(&mut self.active_applet, Applet::Settings, "Settings");
                });
                columns[1].with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::widgets::global_theme_preference_buttons(ui);
                    });
                })
            });
        });

        match self.active_applet {
            Applet::Home => {
                applets::HomeApplet::render(ctx);
            }
            Applet::Notes => {
                applets::NotesApplet::render(ctx);
            }
            _ => {
                applets::DefaultApplet::render(ctx);
            }
        }
    }
}

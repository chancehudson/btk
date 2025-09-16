use anyhow::Result;

mod files;
mod history;
mod home;
mod mail;
mod notes;
mod settings;
mod tasks;

pub use files::FilesApplet;
pub use history::HistoryApplet;
pub use mail::MailApplet;
pub use notes::NotesApplet;
pub use settings::SettingsApplet;
pub use tasks::TasksApplet;

use crate::app::AppEvent;
use crate::data::AppState;

pub struct DefaultApplet;
impl Applet for DefaultApplet {}

pub trait Applet {
    fn init(&mut self, _state: &AppState) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "unimplemented"
    }

    fn handle_app_events(&mut self, _events: &Vec<AppEvent>, _state: &AppState) -> Result<()> {
        Ok(())
    }

    fn render(&mut self, ctx: &egui::Context, _state: &AppState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("unimplemented");
            // Show window info
            if let Some(rect) = ctx.memory(|m| m.area_rect("debug")) {
                ui.label(format!("Size: {:.0} x {:.0}", rect.width(), rect.height()));
            }
        });
    }
}

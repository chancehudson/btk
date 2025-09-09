use super::Applet;

use crate::app_state::AppState;

#[derive(Default)]
pub struct MailApplet;
impl Applet for MailApplet {
    fn name(&self) -> &str {
        "Mail"
    }

    fn render(&mut self, ctx: &egui::Context, _state: &AppState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("actually fuck gmail");
            ui.label("they built a rest api and put pop3/imap behind oauth");
            ui.label("this is a direct attack on information ownership");
            ui.separator();
            ui.label("this is no time for compromise with the morally impoverished");
        });
    }
}

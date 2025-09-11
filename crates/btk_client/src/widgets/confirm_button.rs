/// A button that shows a confirmation message on click. User must click a second time to initiate
/// an action.
use egui::Widget;
use serde::Deserialize;
use serde::Serialize;
use web_time::SystemTime;

/// How long before the confirm text is auto-reverted to the normal text. e.g. click a button then
/// how long until it resets.
const HIDE_CONFIRM_TIMEOUT_SECONDS: f32 = 2.0;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ConfirmButton {
    id: String,
    pub text: String,
    pub confirm_text: String,
    showed_confirm_at: Option<f64>,
    confirmed: bool,
}

impl ConfirmButton {
    pub fn init(id: String, ui: &mut egui::Ui, init: &(dyn Fn(&mut Self))) -> Self {
        ui.ctx()
            .data(|d| d.get_temp(id.clone().into()))
            .unwrap_or_else(|| {
                let mut out = Self {
                    id: id.clone(),
                    ..Default::default()
                };
                init(&mut out);
                ui.ctx().data_mut(|d| d.insert_temp(id.into(), out.clone()));
                out
            })
    }

    /// Determine if the action was confirmed this frame. Analogous to `clicked()` on a normal
    /// button.
    pub fn confirmed(&self) -> bool {
        self.confirmed
    }
}

impl Widget for ConfirmButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut data = ui
            .ctx()
            .data(|d| d.get_temp(self.id.clone().into()))
            .unwrap_or_else(|| Self {
                id: self.id,
                ..Default::default()
            });
        if let Some(showed_confirm_at) = data.showed_confirm_at {
            let now = SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs_f64();
            let elapsed = now - showed_confirm_at;
            if elapsed >= HIDE_CONFIRM_TIMEOUT_SECONDS.into() {
                data.showed_confirm_at = None;
            } else {
                // add a small time buffer for float inaccuracies and frame timing jitter
                ui.ctx().request_repaint_after_secs(
                    0.1 + HIDE_CONFIRM_TIMEOUT_SECONDS - (elapsed as f32),
                );
            }
        }
        data.confirmed = false;
        let out = if data.showed_confirm_at.is_some() {
            let button = ui.button(&data.confirm_text);
            if button.clicked() {
                data.confirmed = true;
                data.showed_confirm_at = None;
            }
            button
        } else {
            let button = ui.button(&data.text);
            if button.clicked() {
                data.showed_confirm_at = Some(
                    SystemTime::now()
                        .duration_since(web_time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs_f64(),
                );
            }
            button
        };
        ui.ctx()
            .data_mut(|d| d.insert_temp(data.id.clone().into(), data));
        out
    }
}

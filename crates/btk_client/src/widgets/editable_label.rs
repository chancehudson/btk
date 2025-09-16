use egui::Widget;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct EditableLabel {
    id: String,
    pub value: String,
    value_edited: String,
    is_editing: bool,
    needs_initial_focus: bool,
    changed: bool,
}

impl EditableLabel {
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

    /// Update the stored value only if it's changed.
    pub fn update_value_if_needed(&mut self, new_value: &str) {
        if self.value != new_value {
            self.value = new_value.to_string();
        }
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}

impl Widget for EditableLabel {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        self.changed = false;
        let out = if self.is_editing {
            let r = ui.horizontal(|ui| {
                ui.add(egui::Label::new("✏ "));
                let r = ui.add(egui::TextEdit::singleline(&mut self.value_edited));
                if self.needs_initial_focus {
                    self.needs_initial_focus = false;
                    r.request_focus();
                }
                if r.has_focus() {
                    r.show_tooltip_ui(|ui| {
                        ui.label("Press Enter to save, Esc to cancel");
                    });
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    self.changed = true;
                    self.value = self.value_edited.clone();
                    self.is_editing = false;
                } else if r.lost_focus() {
                    self.is_editing = false;
                }
                r
            });
            r.response
        } else {
            let r = ui.add(egui::Label::new(&format!("✏ {}", &self.value)));
            if r.clicked() {
                self.is_editing = true;
                self.value_edited = self.value.clone();
                self.needs_initial_focus = true;
            }
            r
        };
        ui.ctx()
            .data_mut(|d| d.insert_temp(self.id.clone().into(), self));
        out
    }
}

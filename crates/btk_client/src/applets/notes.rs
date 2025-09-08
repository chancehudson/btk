use anyhow::Result;
use egui::Frame;
use egui::ScrollArea;
use egui::TextEdit;
use egui::TextStyle;
use egui_commonmark::CommonMarkCache;
use egui_commonmark::CommonMarkViewer;

use super::Applet;
use crate::app::AppState;

#[derive(Default, PartialEq)]
enum LastScrolled {
    #[default]
    Source,
    Rendered,
}

#[derive(Default)]
pub struct NotesApplet {
    md_cache: CommonMarkCache,
    active_note_name: String,
    active_note: String,
    last_scrolled: LastScrolled,
    last_source_offset: f32,
    last_source_percent: f32,
    last_source_height: f32,
    last_rendered_offset: f32,
    last_rendered_percent: f32,
    last_rendered_height: f32,
}

impl NotesApplet {
    fn save(&self, state: AppState) -> Result<()> {
        // state.local_data.db.insert()
        Ok(())
    }
}

impl Applet for NotesApplet {
    fn name(&self) -> &str {
        "Notes"
    }

    fn render(&mut self, ctx: &egui::Context, state: &AppState) {
        // egui::Window::new("scroll debug").show(ctx, |ui| {
        //     ui.label("source");
        //     ui.label(format!(
        //         "offset: {} px, {}%",
        //         self.last_source_offset,
        //         100. * self.last_source_percent
        //     ));
        //     ui.label("rendered");
        //     ui.label(format!(
        //         "offset: {} px, {}%",
        //         self.last_rendered_offset,
        //         100. * self.last_rendered_percent
        //     ));
        // });

        egui::SidePanel::left("notes_list").show(ctx, |ui| {
            ui.label("test");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                let _ = ui.text_edit_singleline(&mut self.active_note_name);
                let _ = ui.button("save");
                let _ = ui.button("share");
                let _ = ui.button("info");
            });
            ui.allocate_ui(ui.available_size(), |ui| {
                let available_height = ui.available_height();
                println!("{available_height}");
                let line_height = ui.text_style_height(&TextStyle::Body);
                let desired_rows = (available_height / line_height) as usize;
                ui.horizontal_top(|ui| {
                    ui.set_height(available_height);
                    Frame::new()
                        .stroke(ui.ctx().style().visuals.widgets.noninteractive.bg_stroke)
                        .fill(ui.ctx().style().visuals.widgets.open.bg_fill)
                        .show(ui, |ui| {
                            ui.set_height(available_height);
                            let scroll_area = ScrollArea::vertical()
                                .vertical_scroll_offset(
                                    if self.last_scrolled == LastScrolled::Source {
                                        self.last_source_offset
                                    } else {
                                        self.last_rendered_percent * self.last_source_height
                                    },
                                )
                                .id_salt("source_scroll_area")
                                .show(ui, |ui| {
                                    ui.set_height(available_height);
                                    TextEdit::multiline(&mut self.active_note)
                                        .frame(false)
                                        .hint_text("Your markdown text here...")
                                        .clip_text(true)
                                        // subtract one to avoid scroll bars on an empty text
                                        // editor :roll_eyes:
                                        .desired_rows(desired_rows.max(1) - 1)
                                        .show(ui);
                                });

                            self.last_source_height =
                                (scroll_area.content_size.y - ui.available_height()).max(0.0);
                            if (scroll_area.state.offset.y - self.last_source_offset).abs() > 0.1 {
                                self.last_source_offset = scroll_area.state.offset.y;
                                self.last_source_percent = if self.last_source_height > 1.0 {
                                    self.last_source_offset / self.last_source_height
                                } else {
                                    0.0
                                };

                                self.last_scrolled = LastScrolled::Source;
                            }
                        });
                    Frame::new()
                        .stroke(ui.ctx().style().visuals.widgets.noninteractive.bg_stroke)
                        .fill(ui.ctx().style().visuals.widgets.open.bg_fill)
                        .inner_margin(3.0)
                        .show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            let scroll_area = ScrollArea::vertical()
                                .vertical_scroll_offset(
                                    if self.last_scrolled == LastScrolled::Source {
                                        self.last_source_percent * self.last_rendered_height
                                    } else {
                                        self.last_rendered_offset
                                    },
                                )
                                .id_salt("rendered_scroll_area")
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    CommonMarkViewer::new().show(
                                        ui,
                                        &mut self.md_cache,
                                        &self.active_note,
                                    );
                                });
                            self.last_rendered_height =
                                (scroll_area.content_size.y - ui.available_height()).max(0.0);
                            if (scroll_area.state.offset.y - self.last_rendered_offset).abs() > 0.1
                            {
                                self.last_rendered_offset = scroll_area.state.offset.y;
                                self.last_rendered_percent =
                                    scroll_area.state.offset.y / self.last_rendered_height;
                                self.last_rendered_percent = if self.last_rendered_height > 1.0 {
                                    self.last_rendered_offset / self.last_rendered_height
                                } else {
                                    0.0
                                };
                                self.last_scrolled = LastScrolled::Rendered;
                            }
                        });
                });
            });
        });
    }
}

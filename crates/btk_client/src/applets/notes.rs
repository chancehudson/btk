use anondb::Journal;
use anyhow::Result;
use egui::Color32;
use egui::Frame;
use egui::ScrollArea;
use egui::TextEdit;
use egui::TextStyle;
use egui_commonmark::CommonMarkCache;
use egui_commonmark::CommonMarkViewer;
use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::prelude::*;

use super::Applet;
use crate::app::AppEvent;
use crate::app::AppState;

#[derive(Default, PartialEq)]
enum LastScrolled {
    #[default]
    Source,
    Rendered,
}

/// Table in anondb reserved for notes applet
const NOTES_TABLE_NAME: &str = "notes";

/// Inputs we sometimes want to explicitly focus
const INPUT_NOTE_NAME: &str = "name_text_input";
const INPUT_NOTE_SOURCE: &str = "source_multiline_input";

#[derive(Default)]
pub struct NotesApplet {
    active_note_name: String,
    active_note_unsaved: String,
    active_note: String,

    _is_showing_history: bool,

    note_names: Vec<String>,

    // render management
    md_cache: CommonMarkCache,

    // scroll management
    last_scrolled: LastScrolled,
    last_source_offset: f32,
    last_source_percent: f32,
    last_source_height: f32,
    last_rendered_offset: f32,
    last_rendered_percent: f32,
    last_rendered_height: f32,
}

impl NotesApplet {
    fn table_name(note_name: &str) -> String {
        format!("note-{}", note_name)
    }

    fn reload_note_names(&mut self, state: &AppState) -> Result<()> {
        self.note_names = state
            .local_data
            .active_cloud()?
            .db
            .find_many::<String, (), _>(NOTES_TABLE_NAME, |_, _| true)
            .unwrap_or(vec![])
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        Ok(())
    }

    fn reset_note_state(&mut self) {
        if self.active_note_unsaved != self.active_note {
            println!("WARNING: unsaved changes");
        }
        self.active_note = String::default();
        self.active_note_unsaved = String::default();
        self.active_note_name = String::default();
    }

    fn open(&mut self, note_name: String, state: &AppState) -> Result<()> {
        if self.active_note_unsaved != self.active_note {
            println!("WARNING: unsaved changes");
            return Ok(());
        }

        // load the diffs and apply them to construct the latest state
        let mut active_note = String::default();
        let tx = state.local_data.active_cloud()?.db.begin_read()?;
        let table = tx.open_table(Journal::table_definition(&Self::table_name(&note_name)))?;
        let mut range = table.range::<anondb::Bytes>(..)?;
        while let Some(entry) = range.next() {
            let (_index_bytes, bytes) = entry?;
            // let index: u64 = index_bytes.value().into();
            // println!("{}", index);
            let bytes = bytes.value();
            let diff = diffy::Patch::from_str((&bytes).into())?;
            // println!("{diff}");
            active_note = diffy::apply(&active_note, &diff)?;
        }

        self.active_note = active_note.clone();
        self.active_note_unsaved = active_note;
        self.active_note_name = note_name;

        Ok(())
    }

    fn save(&mut self, state: &AppState) -> Result<()> {
        if self.active_note_name.is_empty() {
            println!("WARNING: attempting to save note without a name");
            return Ok(());
        }
        // We'll save each note to its own table. Each entry in the table represents a diff from
        // the previous version.
        let mut tx = state.local_data.active_cloud()?.db.begin_write()?;

        // Save our text diff for the current note
        let mut note_table = tx.open_table(&Self::table_name(&self.active_note_name))?;
        let diff = diffy::create_patch(&self.active_note, &self.active_note_unsaved);
        let diff_index = note_table.len()?;
        note_table.insert_bytes(&diff_index.into(), &diff.to_string().into())?;
        drop(note_table);

        // Make sure our note is registered in the list of notes
        let mut note_names_table = tx.open_table(NOTES_TABLE_NAME)?;
        note_names_table.insert(&self.active_note_name, &())?;
        drop(note_names_table);

        tx.commit()?;

        self.active_note = self.active_note_unsaved.clone();

        self.reload_note_names(state)?;

        Ok(())
    }

    fn render_side_list(&mut self, ctx: &egui::Context, state: &AppState) {
        egui::SidePanel::left("notes_list")
            .resizable(true)
            .show(ctx, |ui| {
                egui_taffy::tui(ui, "notes_list_inner")
                    .reserve_available_space()
                    .style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: Some(AlignItems::Stretch),
                        min_size: Size {
                            width: percent(1.0),
                            height: length(0.0),
                        },
                        ..Default::default()
                    })
                    .show(|tui| {
                        tui.style(Style {
                            padding: length(4.0),
                            margin: length(2.0),
                            flex_direction: FlexDirection::Row,
                            align_items: Some(AlignItems::Center),
                            justify_content: Some(JustifyContent::SpaceBetween),
                            ..Default::default()
                        })
                        .add(|tui| {
                            tui.heading("Saved notes");
                            tui.style(Style {
                                padding: length(4.0),
                                margin: length(2.0),
                                ..Default::default()
                            })
                            .ui(|ui| {
                                if ui.button("+").clicked() {
                                    self.reset_note_state();
                                    ctx.memory_mut(|mem| {
                                        mem.request_focus(INPUT_NOTE_NAME.into());
                                    });
                                }
                            });
                        });
                        for name in self.note_names.clone() {
                            if tui
                                .style(Style {
                                    padding: length(4.0),
                                    margin: length(2.0),
                                    ..Default::default()
                                })
                                .selectable(name == self.active_note_name, |tui| {
                                    tui.label(&name);
                                })
                                .clicked()
                            {
                                ctx.memory_mut(|mem| mem.request_focus(INPUT_NOTE_SOURCE.into()));
                                self.open(name.clone(), state)
                                    .expect(&format!("failed to open note: {name}"));
                            }
                        }
                    });
            });
    }
}

impl Applet for NotesApplet {
    fn name(&self) -> &str {
        "Notes"
    }

    fn handle_app_events(&mut self, events: &Vec<AppEvent>, state: &AppState) -> Result<()> {
        for event in events {
            match event {
                AppEvent::ActiveAppletChanged => {
                    self.reload_note_names(state)?;
                }
                AppEvent::ActiveCloudChanged => {
                    self.reload_note_names(state)?;
                    self.reset_note_state();
                }
            }
        }
        Ok(())
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

        for event in state.pending_app_events() {
            match event {
                AppEvent::ActiveAppletChanged => {
                    if self.active_note_name.is_empty() {
                        ctx.memory_mut(|mem| mem.request_focus(INPUT_NOTE_NAME.into()));
                    } else {
                        ctx.memory_mut(|mem| mem.request_focus(INPUT_NOTE_SOURCE.into()));
                    }
                }
                _ => {}
            }
        }

        self.render_side_list(ctx, state);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                let response = egui::TextEdit::singleline(&mut self.active_note_name)
                    .id(INPUT_NOTE_NAME.into())
                    .hint_text("start typing a name like note.md")
                    .show(ui)
                    .response;
                if response.changed() {
                    self.active_note_name = self.active_note_name.trim().to_string();
                    if let Err(_e) = self.open(self.active_note_name.clone(), state) {
                        self.active_note = String::default();
                        self.active_note_unsaved = String::default();
                    }
                }
                if response.has_focus()
                    && self.active_note.is_empty()
                    && self.active_note_name.len() > 3
                {
                    response.show_tooltip_ui(|ui| {
                        ui.label("Press enter to create");
                    });
                }
                if response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && self.active_note_name.len() > 0
                {
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    // TODO: first check if we're overwriting an existing note
                    match self.save(state) {
                        Ok(()) => {
                            ctx.memory_mut(|mem| {
                                mem.request_focus(INPUT_NOTE_SOURCE.into());
                            });
                        }
                        Err(e) => {
                            println!("Error saving new note! {:?}", e);
                        }
                    }
                }
                // if ui
                //     .selectable_label(self.is_showing_history, "history")
                //     .clicked()
                // {
                //     self.is_showing_history = !self.is_showing_history;
                // }
                if self.active_note != self.active_note_unsaved {
                    if ui.button("Save").clicked() {
                        self.save(state).expect("failed to save");
                    }
                    // we have unsaved changes
                    ui.colored_label(Color32::RED, "unsaved changes!");
                }
            });
            ui.allocate_ui(ui.available_size(), |ui| {
                let available_height = ui.available_height();
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
                                    TextEdit::multiline(&mut self.active_note_unsaved)
                                        .id(INPUT_NOTE_SOURCE.into())
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
                                    ui.style_mut().wrap_mode = Default::default();
                                    ui.set_width(ui.available_width());
                                    CommonMarkViewer::new().show(
                                        ui,
                                        &mut self.md_cache,
                                        &self.active_note_unsaved,
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

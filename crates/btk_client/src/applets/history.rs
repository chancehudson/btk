use anondb::JournalTransaction;
use anondb::TransactionOperation;
use anyhow::Result;
use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::Overflow;
use egui_taffy::taffy::Point;
use egui_taffy::taffy::prelude::*;

use crate::app::AppEvent;
use crate::data::AppState;

use super::Applet;

#[derive(Default)]
pub struct HistoryApplet {
    history: Vec<JournalTransaction>,
    showing_create_duplicate_modal: bool,
    duplicate_index: u64,
    duplicate_cloud_name: String,
}

impl HistoryApplet {
    fn render_create_duplicate_modal(&mut self, ctx: &egui::Context, state: &AppState) {
        let modal = egui::Modal::new("create_duplicate_modal".into()).show(ctx, |ui| {
            let active_cloud = state.active_cloud();
            if active_cloud.is_none() {
                println!("WARNING: no active cloud");
                self.showing_create_duplicate_modal = false;
                return;
            }
            let (active_cloud, metadata) = active_cloud.unwrap();
            ui.heading(format!(
                "Share cloud {} at index {}",
                metadata.name, self.duplicate_index
            ));
            let text_edit = egui::TextEdit::singleline(&mut self.duplicate_cloud_name)
                .hint_text("New cloud name");
            let input = ui.add(text_edit);

            if self.duplicate_cloud_name.trim().len() > 3 {
                input.show_tooltip_ui(|ui| {
                    ui.label("press enter to create");
                });
            }

            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.showing_create_duplicate_modal = false;
            }

            if input.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Err(e) = state.duplicate_active_cloud(
                    self.duplicate_index,
                    std::mem::take(&mut self.duplicate_cloud_name),
                ) {
                    println!("Err duplicating: {:?}", e);
                }
                self.showing_create_duplicate_modal = false;
                state.reload_clouds();
            }
            input.request_focus();
        });
        if modal.response.should_close() {
            self.showing_create_duplicate_modal = false;
        }
    }
}

impl Applet for HistoryApplet {
    fn name(&self) -> &str {
        "History"
    }

    fn handle_app_events(&mut self, events: &Vec<AppEvent>, state: &AppState) -> Result<()> {
        for event in events {
            match event {
                AppEvent::ActiveAppletChanged(applet_name) => {
                    if applet_name == self.name() {
                        if let Some((active_cloud, _metadata)) = state.active_cloud() {
                            self.history = active_cloud.db.journal_transactions()?;
                        }
                    }
                }
                AppEvent::ActiveCloudChanged(applet_name) => {
                    if applet_name == self.name() {
                        if let Some((active_cloud, _metadata)) = state.active_cloud() {
                            self.history = active_cloud.db.journal_transactions()?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn render(&mut self, ctx: &egui::Context, state: &AppState) {
        if self.showing_create_duplicate_modal {
            self.render_create_duplicate_modal(ctx, state);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui_taffy::tui(ui, "history")
                .reserve_available_space()
                .style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
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
                    tui.heading("Cloud history");
                    for (i, tx) in self.history.iter().rev().enumerate() {
                        let index = self.history.len() - i;
                        tui.style(Style {
                            flex_direction: FlexDirection::Row,
                            justify_content: Some(JustifyContent::SpaceBetween),
                            padding: length(4.0),
                            margin: length(2.0),
                            ..Default::default()
                        })
                        .add_with_border(|tui| {
                            tui.style(Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: Some(JustifyContent::SpaceBetween),
                                padding: length(4.0),
                                margin: length(2.0),
                                ..Default::default()
                            })
                            .add(|tui| {
                                tui.heading(format!("mutation #{}", index));
                                tui.label(format!(
                                    "last hash: {}",
                                    hex::encode(tx.last_tx_hash).split_off(64 - 20)
                                ));
                                tui.ui(|ui| ui.add_space(4.0));
                                if tui.button(|tui| tui.label("Share")).clicked() {
                                    self.showing_create_duplicate_modal = true;
                                    self.duplicate_index = (index - 1) as u64;
                                    self.duplicate_cloud_name = String::default();
                                }
                            });
                            tui.style(Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: Some(JustifyContent::SpaceBetween),
                                padding: length(4.0),
                                margin: length(2.0),
                                ..Default::default()
                            })
                            .add(|tui| {
                                let mut hidden_operation_count = 0;
                                for operation in &tx.operations {
                                    match operation {
                                        TransactionOperation::Insert { table_name, .. } => {
                                            tui.label(format!("insert 1 key into {table_name}"));
                                        }
                                        TransactionOperation::Remove(table_name, _key) => {
                                            tui.label(format!("remove 1 key from {table_name}"));
                                        }
                                        TransactionOperation::DeleteTable(table_name) => {
                                            tui.label(format!("delete table {table_name}"));
                                        }
                                        _ => hidden_operation_count += 1,
                                    }
                                }
                                tui.label(format!("{} hidden operations", hidden_operation_count));
                            });
                        });
                    }
                });
        });
    }
}

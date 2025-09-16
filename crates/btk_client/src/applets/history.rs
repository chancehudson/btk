use anondb::JournalTransaction;
use anondb::TransactionOperation;
use anyhow::Result;
use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::Overflow;
use egui_taffy::taffy::Point;
use egui_taffy::taffy::prelude::*;

use crate::app::AppEvent;
use crate::data::AppState;
use crate::widgets::ConfirmButton;

use super::Applet;

#[derive(Default)]
pub struct HistoryApplet {
    history: Vec<JournalTransaction>,
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

    fn render(&mut self, ctx: &egui::Context, _state: &AppState) {
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
                                    hex::encode(tx.last_tx_hash).split_off(20)
                                ));
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
                            tui.style(Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: Some(JustifyContent::SpaceBetween),
                                padding: length(4.0),
                                margin: length(2.0),
                                ..Default::default()
                            })
                            .add(|tui| {
                                tui.ui(|ui| {
                                    let button = ConfirmButton::init(
                                        hex::encode(tx.last_tx_hash),
                                        ui,
                                        &|b| {
                                            b.text = "Duplicate cloud at this point".to_string();
                                            b.confirm_text = "Not implemented yet :(".to_string();
                                        },
                                    );
                                    ui.add(button);
                                });
                            });
                        });
                    }
                });
        });
    }
}

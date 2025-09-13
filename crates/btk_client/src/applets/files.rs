use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::Overflow;
use egui_taffy::taffy::Point;
use egui_taffy::taffy::prelude::*;

use super::Applet;
use crate::data::AppState;

#[derive(Default)]
pub struct FilesApplet {}

impl Applet for FilesApplet {
    fn name(&self) -> &str {
        "Files"
    }

    fn render(&mut self, ctx: &egui::Context, state: &AppState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Files");
            });
            egui_taffy::tui(ui, "home")
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
                .show(|tui| {});
        });
    }
}

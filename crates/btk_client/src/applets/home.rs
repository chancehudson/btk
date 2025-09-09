use egui::Color32;
use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::prelude::*;

use super::Applet;
use crate::app::AppState;

#[derive(Default)]
pub struct HomeApplet {}

impl Applet for HomeApplet {
    fn name(&self) -> &str {
        "Home"
    }

    fn render(&mut self, ctx: &egui::Context, state: &AppState) {
        egui::TopBottomPanel::bottom("home_footer").show(ctx, |ui| {
            ctx.style_mut(|style| {
                style.wrap_mode = Some(egui::TextWrapMode::Extend);
            });
            egui_taffy::tui(ui, "home_footer")
                .reserve_available_width()
                .style(Style {
                    min_size: Size {
                        width: percent(1.0),
                        height: auto(),
                    },
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: Some(JustifyContent::Center),
                    flex_wrap: FlexWrap::Wrap,
                    gap: length(100.0),
                    ..Default::default()
                })
                .show(|tui| {
                    tui.style(Style {
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    })
                    .add(|tui| {
                        tui.heading("BTK");
                        tui.label("A local first productivity suite.");
                        tui.label("An encrypted cloud just for you.");
                        tui.label("An attack on surveillance culture.");
                        tui.label("An offering for entropy.");
                        tui.separator();
                        tui.label("The righteous prevail or nothing remains.");
                    });
                    tui.style(Style {
                        flex_direction: FlexDirection::Column,
                        align_items: Some(AlignItems::Center),
                        ..Default::default()
                    })
                    .add(|tui| {
                        let image = egui::Image::new(egui::include_image!("../../assets/btk.jpg"));
                        tui.style(Style {
                            size: Size {
                                height: length(100.0),
                                width: length(100.0),
                            },
                            padding: length(4.0),
                            ..Default::default()
                        })
                        .ui_add(image);
                        tui.label("big tech killer, circa 2025");
                    });
                });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui_taffy::tui(ui, "home")
                .reserve_available_space()
                .style(Style {
                    ..Default::default()
                })
                .show(|tui| match state.local_data.list_clouds() {
                    Ok(clouds) => {
                        for cloud in clouds {
                            tui.label("cloud");
                        }
                    }
                    Err(e) => {
                        tui.colored_label(Color32::RED, format!("error loading clouds: {e}"));
                    }
                });
        });
    }
}

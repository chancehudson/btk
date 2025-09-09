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
        self.render_footer(ctx, state);
        egui::CentralPanel::default().show(ctx, |ui| {
            egui_taffy::tui(ui, "home")
                .reserve_available_space()
                .style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    min_size: Size {
                        width: percent(1.0),
                        height: length(0.0),
                    },
                    ..Default::default()
                })
                .show(|tui| {
                    tui.style(Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: Some(JustifyContent::SpaceAround),
                        min_size: Size {
                            width: percent(1.0),
                            height: length(0.0),
                        },
                        ..Default::default()
                    })
                    .add(|tui| {
                        tui.heading("Your encrypted clouds");
                        tui.ui(|ui| {
                            if ui.button("+").clicked() {
                                state
                                    .local_data
                                    .create_cloud()
                                    .expect("failed to create cloud");
                                state.reload_clouds();
                            }
                        });
                    });
                    tui.separator();
                    for cloud in state.local_data.clouds.values() {
                        tui.style(Style {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: Some(JustifyContent::SpaceAround),
                            min_size: Size {
                                width: percent(1.0),
                                height: length(0.0),
                            },
                            ..Default::default()
                        })
                        .add(|tui| {
                            tui.label(&format!("cloud id: {:?}", cloud.id_hex()));
                            tui.ui(|ui| {
                                if let Some(cloud_id) = state.local_data.active_cloud_id
                                    && cloud.id() == &cloud_id
                                {
                                    ui.label("active!");
                                } else {
                                    if ui.button("Activate").clicked() {
                                        state.switch_cloud(*cloud.id());
                                    }
                                }
                            });
                        });
                    }
                });
        });
    }
}

impl HomeApplet {
    fn render_footer(&mut self, ctx: &egui::Context, state: &AppState) {
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
    }
}

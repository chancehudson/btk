/// This widget is superseded by the clouds menu in app.rs
///
///
use std::collections::HashSet;
use std::sync::Arc;

use egui_taffy::Tui;
use egui_taffy::TuiBuilderLogic;
use egui_taffy::taffy::Overflow;
use egui_taffy::taffy::Point;
use egui_taffy::taffy::prelude::*;

use super::Applet;
use crate::data::AppState;
use crate::data::*;

#[derive(Default)]
pub struct HomeApplet {
    showing_private_key: HashSet<[u8; 32]>,
}

impl HomeApplet {
    fn render_cloud_cell(
        &mut self,
        (cloud, metadata): &(Arc<Cloud>, CloudMetadata),
        tui: &mut Tui,
        state: &AppState,
    ) {
        tui.style(Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(JustifyContent::SpaceAround),
            align_items: Some(AlignItems::FlexStart),
            min_size: Size {
                width: percent(1.0),
                height: length(40.0),
            },
            ..Default::default()
        })
        .add(|tui| {
            tui.style(Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            })
            .add_with_border(|tui| {
                tui.heading(&format!("{}", metadata.name));
                tui.label(&format!("created at: {}", metadata.created_at));
                tui.label(&format!("cloud id: {}", cloud.id_hex()));
                if self.showing_private_key.contains(cloud.id()) {
                    tui.label(&format!("cloud key: {}", hex::encode(cloud.private_key())));
                } else {
                    tui.ui(|ui| {
                        if ui.button("show private key").clicked() {
                            self.showing_private_key.insert(*cloud.id());
                        }
                    });
                }
            });
            tui.style(Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            })
            .add(|tui| {
                if let Some(cloud_id) = state.active_cloud_id
                    && cloud.id() == &cloud_id
                {
                    tui.label("active!");
                } else {
                    tui.ui(|ui| {
                        if ui.button("Switch").clicked() {
                            state.switch_cloud(Some(*cloud.id()));
                        }
                    });
                }
            });
        });
    }
}

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
                    tui.style(Style {
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    })
                    .add(|tui| {
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
                                    state.create_cloud(None).expect("failed to create cloud");
                                    state.reload_clouds();
                                }
                            });
                        });
                        tui.separator();
                        for cloud in &state.sorted_clouds {
                            self.render_cloud_cell(cloud, tui, state);
                        }
                    });
                });
        });
    }
}

impl HomeApplet {
    fn render_footer(&mut self, ctx: &egui::Context, _state: &AppState) {
        egui::TopBottomPanel::bottom("home_footer").show(ctx, |ui| {
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

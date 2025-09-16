use std::sync::Arc;

use eframe::egui::FontData;
use eframe::egui::FontDefinitions;
use eframe::egui::FontFamily;

pub fn setup_themes(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Load Inter Variable font
    fonts.font_data.insert(
        "InterVariable".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/InterVariable.ttf"
        ))),
    );

    // Set font fallback chain: Inter first, then egui's default fonts for icons/symbols
    fonts.families.insert(
        FontFamily::Name("Inter".into()),
        vec![
            "InterVariable".to_owned(),
            // Fallback to egui's built-in fonts for missing glyphs (icons, symbols, etc.)
            "Ubuntu-Light".to_owned(),
            "NotoEmoji-Regular".to_owned(),
            "emoji-icon-font".to_owned(),
        ],
    );

    // Also update the main Proportional family to include fallbacks
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .clear();
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .extend(vec![
            "InterVariable".to_owned(),
            "Ubuntu-Light".to_owned(),
            "NotoEmoji-Regular".to_owned(),
            "emoji-icon-font".to_owned(),
        ]);

    ctx.set_fonts(fonts);

    // Brighten text colors
    // let mut visuals = ctx.style().visuals.clone();
    // visuals.override_text_color = Some(Color32::from_rgb(240, 240, 240));
    // visuals.weak_text_color = Some(Color32::from_rgb(200, 200, 200));

    // ctx.set_visuals(visuals);
}

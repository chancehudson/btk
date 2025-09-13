#[cfg(target_arch = "wasm32")]
pub use tokio_with_wasm as tokio;

#[cfg(not(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
)))]
pub use tokio;

mod app;
mod applets;
mod data;
mod network;
mod widgets;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        run_and_return: false,
        viewport: egui::ViewportBuilder::default()
            .with_resizable(true)
            .with_inner_size([800.0, 1200.0])
            .with_min_inner_size([300.0, 400.0]),
        // .with_icon(
        //     // NOTE: Adding an icon is optional
        //     eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
        //         .expect("Failed to load icon"),
        // ),
        ..Default::default()
    };
    eframe::run_native(
        "BTK",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)?))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(app::App::new(cc)?))),
            )
            .await;
    });
}

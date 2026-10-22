#![forbid(unsafe_code)]
//! Native entry point for the Stars! remake desktop shell.

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(stars_desktop::app::APP_TITLE)
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 420.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "stars-remake",
        native_options,
        Box::new(|_creation_context| Ok(Box::new(stars_desktop::StarsApp::new()?))),
    )
}

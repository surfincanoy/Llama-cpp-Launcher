mod app;
mod config;
mod i18n;
mod server;

const WIN_WIDTH: f32 = 520.0;
const WIN_HEIGHT_MIN: f32 = 800.0;
const WIN_HEIGHT_MAX: f32 = 1000.0;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WIN_WIDTH, WIN_HEIGHT_MIN])
            .with_min_inner_size([WIN_WIDTH, WIN_HEIGHT_MIN])
            .with_max_inner_size([WIN_WIDTH, WIN_HEIGHT_MAX])
            .with_title("llama.cpp Launcher"),
        ..Default::default()
    };
    eframe::run_native(
        "llamacpp-launcher",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}

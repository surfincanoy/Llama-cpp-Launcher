mod app;
mod config;
mod i18n;
mod server;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 800.0])
            .with_min_inner_size([520.0, 800.0])
            .with_max_inner_size([520.0, 1000.0])
            .with_title("llama.cpp Launcher"),
        ..Default::default()
    };
    eframe::run_native(
        "llamacpp-launcher",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}

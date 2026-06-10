mod app;
mod config;
mod i18n;
mod server;

fn load_icon() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("icon/llamacpp-launcher.png"))
        .expect("图标文件加载失败");
    let size = 64u32;
    let small = img.resize_exact(size, size, image::imageops::FilterType::CatmullRom);
    let rgba = small.to_rgba8().into_raw();
    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 700.0])
            .with_icon(load_icon())
            .with_title("llama.cpp Launcher"),
        ..Default::default()
    };
    eframe::run_native(
        "llamacpp-launcher",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}

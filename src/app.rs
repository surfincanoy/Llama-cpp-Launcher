use std::sync::mpsc;

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, RichText, Vec2};
use crate::config::{self, Config};
use crate::i18n::L10n;
use crate::server::{self, ServerEvent, ServerProcess};

const ACCENT: Color32 = Color32::from_rgb(99, 102, 241);
const DANGER: Color32 = Color32::from_rgb(239, 68, 68);
const SURFACE: Color32 = Color32::from_rgb(30, 30, 40);
const SURFACE_LIGHT: Color32 = Color32::from_rgb(40, 40, 55);
const BG: Color32 = Color32::from_rgb(22, 22, 30);
const TEXT: Color32 = Color32::from_rgb(220, 220, 230);
const TEXT_DIM: Color32 = Color32::from_rgb(140, 140, 160);
const GREEN: Color32 = Color32::from_rgb(0, 200, 83);
const RED: Color32 = Color32::from_rgb(239, 68, 68);

pub struct App {
    config: Config,
    models: Vec<String>,
    server_process: Option<ServerProcess>,
    logs: Vec<String>,
    status: String,
    event_receiver: Option<mpsc::Receiver<ServerEvent>>,
    l10n: L10n,
}

impl Default for App {
    fn default() -> Self {
        let config = config::load_config();
        let models = if !config.model_dir.is_empty() {
            config::list_models(&config.model_dir)
        } else {
            Vec::new()
        };
        let l10n = L10n::detect();
        Self {
            config,
            models,
            server_process: None,
            logs: Vec::new(),
            status: l10n.not_running().to_string(),
            event_receiver: None,
            l10n,
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_chinese_fonts(&cc.egui_ctx);
        setup_visuals(&cc.egui_ctx);
        Self::default()
    }

    fn add_log(&mut self, msg: &str) {
        self.logs.push(msg.to_string());
        if self.logs.len() > 200 {
            self.logs.drain(0..self.logs.len() - 200);
        }
    }

    fn is_running(&self) -> bool {
        self.server_process.is_some()
    }

    fn poll_events(&mut self) {
        let events: Vec<ServerEvent> = if let Some(receiver) = self.event_receiver.as_ref() {
            let mut evts = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                evts.push(event);
            }
            evts
        } else {
            return;
        };

        for event in events {
            match event {
                ServerEvent::Log(msg) => self.add_log(&msg),
                ServerEvent::Started(process) => {
                    self.status = self.l10n.running_with_pid(process.pid());
                    self.server_process = Some(process);
                    self.event_receiver = None;
                    let url = format!("http://{}:{}", self.config.host, self.config.port);
                    let _ = webbrowser::open(&url);
                }
                ServerEvent::Failed(msg) => {
                    self.add_log(&self.l10n.failed(&msg));
                    self.status = self.l10n.start_failed().to_string();
                    self.event_receiver = None;
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);

            ui.add_space(8.0);

            // Config card
            egui::Frame::new()
                .fill(SURFACE)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(self.l10n.config_title())
                            .size(14.0)
                            .strong()
                            .color(ACCENT),
                    );
                    ui.add_space(8.0);

                    egui::Grid::new("config_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new(self.l10n.executable()).color(TEXT_DIM));
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.config.executable)
                                        .desired_width(280.0)
                                        .hint_text(self.l10n.exec_hint()),
                                );
                                if ui.add(
                                    egui::Button::new(RichText::new(self.l10n.browse()).color(TEXT))
                                        .fill(SURFACE_LIGHT)
                                        .corner_radius(CornerRadius::same(4)),
                                ).clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter(self.l10n.exec_filter(), &["exe", "bin", ""])
                                        .pick_file()
                                    {
                                        self.config.executable = path.to_string_lossy().to_string();
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label(RichText::new(self.l10n.model_dir()).color(TEXT_DIM));
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.config.model_dir)
                                        .desired_width(280.0)
                                        .hint_text(self.l10n.dir_hint()),
                                );
                                if ui.add(
                                    egui::Button::new(RichText::new(self.l10n.browse()).color(TEXT))
                                        .fill(SURFACE_LIGHT)
                                        .corner_radius(CornerRadius::same(4)),
                                ).clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        self.config.model_dir = path.to_string_lossy().to_string();
                                        self.models = config::list_models(&self.config.model_dir);
                                        if self.config.model_name.is_empty() {
                                            self.config.model_name =
                                                self.models.first().cloned().unwrap_or_default();
                                        }
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label(RichText::new(self.l10n.model_file()).color(TEXT_DIM));
                            egui::ComboBox::from_id_salt("model_select")
                                .selected_text(
                                    RichText::new(&self.config.model_name).color(TEXT),
                                )
                                .width(290.0)
                                .show_ui(ui, |ui| {
                                    for model in &self.models {
                                        ui.selectable_value(
                                            &mut self.config.model_name,
                                            model.clone(),
                                            RichText::new(model).color(TEXT),
                                        );
                                    }
                                });
                            ui.end_row();

                            ui.label(RichText::new(self.l10n.host()).color(TEXT_DIM));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.config.host)
                                    .desired_width(160.0)
                                    .font(egui::TextStyle::Monospace),
                            );
                            ui.end_row();

                            ui.label(RichText::new(self.l10n.port()).color(TEXT_DIM));
                            ui.add(
                                egui::DragValue::new(&mut self.config.port)
                                    .range(1024..=65535)
                                    .prefix("  ")
                                    .suffix("  "),
                            );
                            ui.end_row();

                            ui.label(RichText::new(self.l10n.gpu_layers()).color(TEXT_DIM));
                            ui.add(
                                egui::DragValue::new(&mut self.config.n_gpu_layers)
                                    .range(0..=1000)
                                    .prefix("  ")
                                    .suffix("  "),
                            );
                            ui.end_row();

                            ui.label(RichText::new(self.l10n.ctx_size()).color(TEXT_DIM));
                            ui.add(
                                egui::DragValue::new(&mut self.config.ctx_size)
                                    .range(512..=131072)
                                    .prefix("  ")
                                    .suffix("  "),
                            );
                            ui.end_row();
                        });

                    ui.add_space(8.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        if ui.add(
                            egui::Button::new(RichText::new(self.l10n.save_config()).color(TEXT))
                                .fill(SURFACE_LIGHT)
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(120.0, 30.0)),
                        ).clicked() {
                            config::save_config(&self.config);
                        }
                    });
                });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                // Control card
                let control = egui::Frame::new()
                    .fill(SURFACE)
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        let running = self.is_running();
                        let waiting = self.event_receiver.is_some();
                        let start_enabled = !running
                            && !waiting
                            && !self.config.executable.is_empty()
                            && !self.config.model_name.is_empty();
                        let stop_enabled = running && self.event_receiver.is_none();

                        ui.horizontal(|ui| {
                            ui.add_enabled_ui(start_enabled, |ui| {
                                let btn = egui::Button::new(
                                    RichText::new(self.l10n.start_server()).color(Color32::WHITE).strong(),
                                )
                                .fill(if start_enabled { GREEN } else { SURFACE_LIGHT })
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(110.0, 28.0));
                                if ui.add(btn).clicked() {
                                    let msg = self.l10n.starting_server().to_string();
                                    self.logs.clear();
                                    self.add_log(&msg);
                                    self.status = self.l10n.starting().to_string();
                                    let (tx, rx) = mpsc::channel();
                                    self.event_receiver = Some(rx);
                                    let cfg = self.config.clone();
                                    let lang = self.l10n.lang();
                                    server::start_server_async(
                                        cfg.executable,
                                        cfg.model_dir,
                                        cfg.model_name,
                                        cfg.host,
                                        cfg.port,
                                        cfg.n_gpu_layers,
                                        cfg.ctx_size,
                                        lang,
                                        tx,
                                    );
                                }
                            });

                            ui.add_space(12.0);

                            ui.add_enabled_ui(stop_enabled, |ui| {
                                let btn = egui::Button::new(
                                    RichText::new(self.l10n.stop_server()).color(Color32::WHITE).strong(),
                                )
                                .fill(if stop_enabled { DANGER } else { SURFACE_LIGHT })
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(110.0, 28.0));
                            if ui.add(btn).clicked() {
                                let msg = self.l10n.stopping_server().to_string();
                                self.add_log(&msg);
                                let lang = self.l10n.lang();
                                if let Some(ref mut proc) = self.server_process {
                                    let stop_logs = server::stop_server(proc, lang);
                                        for log in &stop_logs {
                                            self.add_log(log);
                                        }
                                    }
                                self.server_process = None;
                                self.status = self.l10n.not_running().to_string();
                                }
                            });
                        });
                    });

                let control_height = control.response.rect.height();
                let remaining = ui.available_width();
                ui.allocate_ui_with_layout(
                    Vec2::new(remaining, control_height),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        egui::Frame::new()
                            .fill(SURFACE)
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(16, 8))
                            .show(ui, |ui| {
                                let (dot_color, label_color) = if self.is_running() {
                                    (GREEN, TEXT)
                                } else if self.status == self.l10n.start_failed() {
                                    (RED, TEXT)
                                } else {
                                    (TEXT_DIM, TEXT_DIM)
                                };
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::splat(10.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter()
                                        .circle_filled(rect.center(), 5.0, dot_color);
                                    ui.label(
                                        RichText::new(&self.status)
                                            .color(label_color)
                                            .size(13.0),
                                    );
                                });
                            });
                    });
            });

            ui.add_space(10.0);

            // Log area
            ui.label(self.l10n.logs());
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    for line in &self.logs {
                        ui.label(egui::RichText::new(line).monospace());
                    }
                });

            ui.add_space(8.0);
        });
    }
}

fn setup_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = SURFACE;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    ctx.set_style(style);
}

fn setup_chinese_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let font_paths = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    ];

    let mut loaded = false;
    for path in &font_paths {
        if let Ok(data) = std::fs::read(path) {
            fonts.font_data.insert(
                "chinese".to_owned(),
                std::sync::Arc::new(FontData::from_owned(data)),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "chinese".to_owned());
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "chinese".to_owned());
            loaded = true;
            eprintln!("已加载中文字体: {}", path);
            break;
        }
    }

    if !loaded {
        eprintln!("警告: 未找到中文字体，中文可能显示为方框");
    }

    ctx.set_fonts(fonts);
}

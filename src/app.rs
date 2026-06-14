use std::collections::HashMap;
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
const GREEN_DIM: Color32 = Color32::from_rgb(0, 150, 60);
const GREEN_BG: Color32 = Color32::from_rgb(25, 65, 40);
const ORANGE_BG: Color32 = Color32::from_rgb(180, 90, 20);
const RED: Color32 = Color32::from_rgb(239, 68, 68);

pub struct App {
    config: Config,
    models: Vec<String>,
    mmproj_models: Vec<String>,
    server_process: Option<ServerProcess>,
    logs: Vec<String>,
    status: String,
    event_receiver: Option<mpsc::Receiver<ServerEvent>>,
    l10n: L10n,
    port_text: String,
    gpu_layers_text: String,
    ctx_size_text: String,
    spec_nmax_text: String,
    models_max_text: String,
    command_text: String,
    profiles: HashMap<String, Config>,
    current_profile: String,
    loaded_profile: String,
    show_save_dialog: bool,
    save_dialog_name: String,
    show_load_dialog: bool,
}

impl Default for App {
    fn default() -> Self {
        let (profiles, last_profile) = config::load_profiles();
        let (config, command_text) = if !last_profile.is_empty() {
            if let Some(cfg) = profiles.get(&last_profile) {
                (cfg.clone(), cfg.command_text.clone())
            } else {
                (Config::default(), String::new())
            }
        } else {
            (Config::default(), String::new())
        };
        let models = if !config.model_dir.is_empty() {
            config::list_models(&config.model_dir)
        } else {
            Vec::new()
        };
        let mmproj_models = if !config.model_dir.is_empty() {
            config::list_mmproj_models(&config.model_dir)
        } else {
            Vec::new()
        };
        let l10n = L10n::detect();
        let port_text = config.port.to_string();
        let gpu_layers_text = config.n_gpu_layers.to_string();
        let ctx_size_text = config.ctx_size.to_string();
        let spec_nmax_text = config.spec_draft_n_max.to_string();
        let models_max_text = config.models_max.to_string();
        let app = Self {
            config,
            models,
            mmproj_models,
            server_process: None,
            logs: Vec::new(),
            status: l10n.not_running().to_string(),
            event_receiver: None,
            l10n,
            port_text,
            gpu_layers_text,
            ctx_size_text,
            spec_nmax_text,
            models_max_text,
            command_text,
            profiles,
            current_profile: last_profile.clone(),
            loaded_profile: last_profile,
            show_save_dialog: false,
            save_dialog_name: String::new(),
            show_load_dialog: false,
        };
        app.sync_profile_sections();
        app
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

    fn sync_profile_sections(&self) {
        for (name, cfg) in &self.profiles {
            if self.models.contains(name) {
                let extra = config::extract_extra_args(&cfg.command_text);
                config::add_config_ini_section_if_missing(name, cfg, &extra);
            }
        }
    }

    fn apply_config(&mut self, profile_name: &str, cfg: &Config) {
        let old_dir = std::mem::take(&mut self.config.model_dir);
        self.config = cfg.clone();
        self.port_text = cfg.port.to_string();
        self.gpu_layers_text = cfg.n_gpu_layers.to_string();
        self.ctx_size_text = cfg.ctx_size.to_string();
        self.spec_nmax_text = cfg.spec_draft_n_max.to_string();
        self.models_max_text = cfg.models_max.to_string();
        self.command_text = cfg.command_text.clone();
        self.current_profile = profile_name.to_string();
        if self.config.model_dir != old_dir && !self.config.model_dir.is_empty() {
            self.models = config::list_models(&self.config.model_dir);
            self.mmproj_models = config::list_mmproj_models(&self.config.model_dir);
            config::auto_generate_config_ini(&self.config);
        }
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
                    let url = if self.config.route_mode {
                        "http://127.0.0.1:8080".to_string()
                    } else {
                        format!("http://{}:{}", self.config.host, self.config.port)
                    };
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
    fn generate_command(&self) -> String {
        if self.config.route_mode {
                    return format!(
                        "{} --models-preset {}",
                        self.config.executable,
                        config::config_ini_path_str()
                    );
        }
        let mut parts = Vec::new();
        parts.push(self.config.executable.clone());
        if !self.config.model_name.is_empty() {
            let model_name = if self.config.model_name.ends_with(".gguf") {
                self.config.model_name.clone()
            } else {
                format!("{}.gguf", self.config.model_name)
            };
            let model_path = format!("{}/{}", self.config.model_dir.trim_end_matches('/'), model_name);
            parts.push("-m".to_string());
            parts.push(model_path);
        }
        parts.push("--host".to_string());
        parts.push(self.config.host.clone());
        parts.push("--port".to_string());
        parts.push(self.port_text.clone());
        parts.push("-c".to_string());
        parts.push(self.ctx_size_text.clone());
        parts.push("--n-gpu-layers".to_string());
        parts.push(self.gpu_layers_text.clone());
        if self.config.mtp_enabled {
            parts.push("--spec-type".to_string());
            parts.push("draft-mtp".to_string());
            parts.push("--spec-draft-n-max".to_string());
            parts.push(self.spec_nmax_text.clone());
        }
        if self.config.flash_attn != "auto" {
            parts.push("--flash-attn".to_string());
            parts.push(self.config.flash_attn.clone());
        }
        if !self.config.mmproj.is_empty() {
            let mmproj_name = if self.config.mmproj.ends_with(".gguf") {
                self.config.mmproj.clone()
            } else {
                format!("{}.gguf", self.config.mmproj)
            };
            let mmproj_path = format!("{}/{}", self.config.model_dir.trim_end_matches('/'), mmproj_name);
            parts.push("--mmproj".to_string());
            parts.push(mmproj_path);
        }
        parts.join(" ")
    }

    fn parse_command(&mut self, cmd: &str) {
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "-m" => {
                    i += 1;
                }
                "--host" => {
                    if let Some(v) = tokens.get(i + 1) {
                        self.config.host = v.to_string();
                        i += 1;
                    }
                }
                "--port" => {
                    if let Some(v) = tokens.get(i + 1) {
                        self.port_text = v.to_string();
                        i += 1;
                    }
                }
                "-c" => {
                    if let Some(v) = tokens.get(i + 1) {
                        self.ctx_size_text = v.to_string();
                        i += 1;
                    }
                }
                "--n-gpu-layers" => {
                    if let Some(v) = tokens.get(i + 1) {
                        self.gpu_layers_text = v.to_string();
                        i += 1;
                    }
                }
                "--spec-type" => {
                    self.config.mtp_enabled = tokens.get(i + 1).is_some_and(|&v| v == "draft-mtp");
                    i += 1;
                }
                "--spec-draft-n-max" => {
                    if let Some(v) = tokens.get(i + 1) {
                        self.spec_nmax_text = v.to_string();
                        i += 1;
                    }
                }
                "--flash-attn" | "-fa" => {
                    if let Some(v) = tokens.get(i + 1) {
                        if !v.starts_with('-') {
                            self.config.flash_attn = v.to_string();
                            i += 1;
                        }
                    }
                }
                x if x.starts_with("--flash-attn=") => {
                    if let Some(v) = x.split('=').nth(1) {
                        self.config.flash_attn = v.to_string();
                    }
                }
                "--mmproj" => {
                    if let Some(v) = tokens.get(i + 1) {
                        self.config.mmproj = v.to_string();
                        i += 1;
                    }
                }
                "--models-preset" => {
                    self.config.route_mode = true;
                    i += 1;
                }
                "--models-max" => {
                    if let Some(v) = tokens.get(i + 1) {
                        self.models_max_text = v.to_string();
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn extract_extra_args(&self) -> String {
        config::extract_extra_args(&self.command_text)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();

        // Sync text edits → config
        if let Ok(v) = self.port_text.parse::<u16>() {
            self.config.port = v;
        }
        if let Ok(v) = self.gpu_layers_text.parse::<u32>() {
            self.config.n_gpu_layers = v;
        }
        if let Ok(v) = self.ctx_size_text.parse::<u32>() {
            self.config.ctx_size = v;
        }
        if let Ok(v) = self.spec_nmax_text.parse::<u32>() {
            self.config.spec_draft_n_max = v;
        }
        if let Ok(v) = self.models_max_text.parse::<u32>() {
            self.config.models_max = v;
        }

        let cmd_id = egui::Id::new("cmd_text_area");
        let cmd_has_focus = ctx.memory(|m| m.focused() == Some(cmd_id));
        if !cmd_has_focus {
            let generated = self.generate_command();
            if self.command_text != generated {
                self.command_text = generated;
            }
        } else {
            let cmd = self.command_text.clone();
            self.parse_command(&cmd);
        }

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
                                        self.mmproj_models = config::list_mmproj_models(&self.config.model_dir);
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

                            ui.label(RichText::new(self.l10n.vision_model()).color(TEXT_DIM));
                            egui::ComboBox::from_id_salt("mmproj_select")
                                .selected_text(
                                    RichText::new(
                                        if self.config.mmproj.is_empty() { self.l10n.none() } else { &self.config.mmproj }
                                    ).color(TEXT),
                                )
                                .width(290.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.config.mmproj,
                                        String::new(),
                                        RichText::new(self.l10n.none()).color(TEXT_DIM),
                                    );
                                    for model in &self.mmproj_models {
                                        ui.selectable_value(
                                            &mut self.config.mmproj,
                                            model.clone(),
                                            RichText::new(model).color(TEXT),
                                        );
                                    }
                                });
                            ui.end_row();
                        });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            egui::Grid::new("left_grid")
                                .num_columns(2)
                                .spacing([12.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(RichText::new(self.l10n.host()).color(TEXT_DIM));
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.config.host)
                                            .desired_width(110.0)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                    ui.end_row();

                                    ui.label(RichText::new(self.l10n.port()).color(TEXT_DIM));
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.port_text)
                                            .desired_width(110.0)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                    ui.end_row();

                                    ui.label(RichText::new(self.l10n.gpu_layers()).color(TEXT_DIM));
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.gpu_layers_text)
                                            .desired_width(110.0)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                    ui.end_row();

                                    ui.label(RichText::new(self.l10n.ctx_size()).color(TEXT_DIM));
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.ctx_size_text)
                                            .desired_width(110.0)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                    ui.end_row();
                                });
                        });
                        ui.add_space(24.0);
                        ui.vertical(|ui| {
                            egui::Grid::new("right_grid")
                                .num_columns(2)
                                .spacing([12.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(RichText::new(self.l10n.models_max()).color(TEXT_DIM));
                                    ui.add_enabled(self.config.route_mode,
                                        egui::TextEdit::singleline(&mut self.models_max_text)
                                            .desired_width(60.0)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                    ui.end_row();

                                    ui.checkbox(&mut self.config.mtp_enabled,
                                        RichText::new(self.l10n.mtp()).color(TEXT_DIM).size(13.0));
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.spec_nmax_text)
                                            .desired_width(60.0)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                    ui.end_row();

                                    ui.label(RichText::new(self.l10n.flash_attn()).color(TEXT_DIM).size(13.0));
                                    egui::ComboBox::from_id_salt("flash_attn")
                                        .selected_text(RichText::new(&self.config.flash_attn).color(TEXT))
                                        .width(60.0)
                                        .show_ui(ui, |ui| {
                                            for v in &["auto", "on", "off"] {
                                                ui.selectable_value(
                                                    &mut self.config.flash_attn,
                                                    v.to_string(),
                                                    RichText::new(*v).color(TEXT),
                                                );
                                            }
                                        });
                                    ui.end_row();
                                });
                        });
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("⌨ {}", self.l10n.cmd_args()))
                            .size(13.0)
                            .color(ACCENT),
                    );
                    ui.add_sized(
                        Vec2::new(ui.available_width(), 100.0),
                        |ui: &mut egui::Ui| {
                            let mut display_text = if self.config.route_mode {
                                format!("--models-preset {}", config::config_ini_path_str())
                            } else {
                                self.command_text.clone()
                            };
                            let te = egui::TextEdit::multiline(&mut display_text)
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(4)
                                .id(cmd_id)
                                .interactive(!self.config.route_mode);
                            let resp = ui.add(te);
                            if !self.config.route_mode && resp.changed() {
                                self.command_text = display_text;
                            }
                            resp
                        },
                    );

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let (dot_color, label_color) = if self.is_running() {
                            (GREEN, TEXT)
                        } else if self.status == self.l10n.start_failed() {
                            (RED, TEXT)
                        } else {
                            (TEXT_DIM, TEXT_DIM)
                        };
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
                        ui.add_space(16.0);
                        let prev_w = ui.style().spacing.icon_width;
                        let prev_iw = ui.style().spacing.icon_width_inner;
                        ui.style_mut().spacing.icon_width = 14.0;
                        ui.style_mut().spacing.icon_width_inner = 6.0;
                        let resp = ui.add(egui::RadioButton::new(self.config.route_mode,
                            RichText::new(self.l10n.route_mode()).color(TEXT_DIM).size(13.0)));
                        ui.style_mut().spacing.icon_width = prev_w;
                        ui.style_mut().spacing.icon_width_inner = prev_iw;
                        if resp.clicked() {
                            self.config.route_mode = !self.config.route_mode;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(
                                egui::Button::new(RichText::new(self.l10n.load_config()).color(TEXT))
                                    .fill(SURFACE_LIGHT)
                                    .corner_radius(CornerRadius::same(6))
                                    .min_size(Vec2::new(120.0, 30.0)),
                            ).clicked() {
                                self.show_load_dialog = true;
                            }
                            ui.add_space(8.0);
                            if ui.add(
                                egui::Button::new(RichText::new(self.l10n.save_config()).color(TEXT))
                                    .fill(SURFACE_LIGHT)
                                    .corner_radius(CornerRadius::same(6))
                                    .min_size(Vec2::new(120.0, 30.0)),
                            ).clicked() {
                                self.save_dialog_name = self.config.model_name.clone();
                                self.show_save_dialog = true;
                            }
                            ui.add_space(8.0);
                        });
                    });
                });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let gap = 12.0;
                let btn_w = 110.0;
                let total_w = btn_w * 2.0 + gap;
                ui.add_space((ui.available_width() - total_w).max(0.0) / 2.0);

                let running = self.is_running();
                let waiting = self.event_receiver.is_some();
                let start_enabled = !running
                    && !waiting
                    && !self.config.executable.is_empty()
                    && (self.config.route_mode || !self.config.model_name.is_empty());
                let stop_enabled = running && self.event_receiver.is_none();

                ui.add_enabled_ui(start_enabled, |ui| {
                    let btn = egui::Button::new(
                        RichText::new(self.l10n.start_server()).color(Color32::WHITE).strong(),
                    )
                    .fill(if start_enabled { GREEN_DIM } else { SURFACE_LIGHT })
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(110.0, 28.0));
                    if ui.add(btn).clicked() {
                        let extra_args = self.extract_extra_args();
                        if self.config.route_mode {
                            config::auto_generate_config_ini(&self.config);
                        }
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
                            cfg.mtp_enabled,
                            cfg.flash_attn,
                            cfg.spec_draft_n_max,
                            cfg.mmproj,
                            cfg.route_mode,
                            extra_args,
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

            ui.add_space(10.0);

            // Log area
            egui::Frame::new()
                .fill(SURFACE)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.label(self.l10n.logs());
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            for line in &self.logs {
                                if let Some(start) = line.find("http://") {
                                    let end = start + line[start..].find(|c: char| c.is_whitespace()).unwrap_or(line.len() - start);
                                    let url = &line[start..end];
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        if start > 0 {
                                            ui.label(egui::RichText::new(&line[..start]).monospace());
                                        }
                                        ui.hyperlink_to(
                                            egui::RichText::new(url).monospace(),
                                            url,
                                        );
                                        if end < line.len() {
                                            ui.label(egui::RichText::new(&line[end..]).monospace());
                                        }
                                    });
                                } else {
                                    ui.label(egui::RichText::new(line).monospace());
                                }
                            }
                        });
                });

            ui.add_space(8.0);
        });

        if self.show_save_dialog {
            egui::Window::new(self.l10n.save_config())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([300.0, 120.0])
                .show(ctx, |ui| {
                    ui.label(RichText::new(self.l10n.profile_name()).color(TEXT_DIM));
                    ui.add_space(4.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.save_dialog_name)
                            .desired_width(280.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    ui.add_space(8.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.horizontal(|ui| {
                            let name = self.save_dialog_name.clone();
                            if ui.add(
                                egui::Button::new(RichText::new(self.l10n.confirm()).color(TEXT))
                                    .fill(SURFACE_LIGHT)
                                    .corner_radius(CornerRadius::same(4))
                                    .min_size(Vec2::new(80.0, 26.0)),
                            ).clicked() && !name.is_empty() {
                                config::save_profile(&name, &self.config, &name);
                                let extra = self.extract_extra_args();
                                config::update_config_ini_profile(&name, &self.config, &extra);
                                let (profiles, _) = config::load_profiles();
                                self.profiles = profiles;
                                self.current_profile = name;
                                self.show_save_dialog = false;
                            }
                            ui.add_space(12.0);
                            if ui.add(
                                egui::Button::new(RichText::new(self.l10n.cancel()).color(TEXT))
                                    .fill(SURFACE_LIGHT)
                                    .corner_radius(CornerRadius::same(4))
                                    .min_size(Vec2::new(80.0, 26.0)),
                            ).clicked() {
                                self.show_save_dialog = false;
                            }
                        });
                    });
                });
        }

        if self.show_load_dialog {
            let mut close = false;
            let mut load_profile = None;
            let mut delete_profile = None;
            egui::Window::new(self.l10n.load_config())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([340.0, 200.0])
                .show(ctx, |ui| {
                    let profile_names: Vec<String> = self.profiles.keys().cloned().collect();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for name in &profile_names {
                                let bg = if *name == self.loaded_profile {
                                    ORANGE_BG
                                } else if *name == self.config.model_name {
                                    GREEN_BG
                                } else {
                                    SURFACE
                                };
                                ui.horizontal(|ui| {
                                    if ui.add_sized(
                                        Vec2::new(ui.available_width() - 36.0, 28.0),
                                        egui::Button::new(
                                            RichText::new(name).color(TEXT).size(14.0)
                                        )
                                        .fill(bg)
                                        .corner_radius(CornerRadius::same(4)),
                                    ).clicked() {
                                        load_profile = Some(name.clone());
                                        close = true;
                                    }
                                    if ui.add(
                                        egui::Button::new(
                                            RichText::new("🗑").color(TEXT_DIM).size(14.0)
                                        )
                                        .fill(SURFACE)
                                        .corner_radius(CornerRadius::same(4))
                                        .min_size(Vec2::new(28.0, 28.0)),
                                    ).clicked() {
                                        delete_profile = Some(name.clone());
                                    }
                                });
                                ui.add_space(4.0);
                            }
                        });
                    ui.add_space(8.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        if ui.add(
                            egui::Button::new(RichText::new(self.l10n.cancel()).color(TEXT))
                                .fill(SURFACE_LIGHT)
                                .corner_radius(CornerRadius::same(4))
                                .min_size(Vec2::new(80.0, 26.0)),
                        ).clicked() {
                            close = true;
                        }
                    });
                });
            if let Some(name) = delete_profile {
                config::delete_profile(&name);
                let (profiles, _) = config::load_profiles();
                self.profiles = profiles;
                if self.current_profile == name {
                    self.current_profile = String::new();
                }
                if self.loaded_profile == name {
                    self.loaded_profile = String::new();
                }
            }
            if close {
                if let Some(name) = load_profile {
                    if let Some(cfg) = self.profiles.get(&name).cloned() {
                        self.apply_config(&name, &cfg);
                        if self.models.contains(&name) {
                            let extra = self.extract_extra_args();
                            config::add_config_ini_section_if_missing(&name, &self.config, &extra);
                        }
                        self.loaded_profile = name;
                    }
                }
                self.show_load_dialog = false;
            }
        }
    }
}

fn setup_visuals(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = BG;
    v.faint_bg_color = SURFACE;
    v.widgets.noninteractive.bg_fill = SURFACE;
    v.widgets.noninteractive.weak_bg_fill = SURFACE;
    v.widgets.inactive.bg_fill = BG;
    v.widgets.inactive.weak_bg_fill = BG;
    v.widgets.hovered.bg_fill = SURFACE;
    v.widgets.hovered.weak_bg_fill = SURFACE;
    v.widgets.active.bg_fill = SURFACE_LIGHT;
    v.widgets.active.weak_bg_fill = SURFACE_LIGHT;
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    ctx.set_style(style);
}

fn setup_chinese_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let font_paths = [
        // Linux
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        // Windows
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\simsun.ttc",
        // macOS
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
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
            break;
        }
    }

    if !loaded {
        eprintln!("警告: 未找到中文字体，中文可能显示为方框");
    }

    ctx.set_fonts(fonts);
}

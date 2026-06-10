use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub executable: String,
    pub model_dir: String,
    pub model_name: String,
    pub host: String,
    pub port: u16,
    pub n_gpu_layers: u32,
    pub ctx_size: u32,
    pub mtp_enabled: bool,
    pub flash_attn: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            executable: String::new(),
            model_dir: String::new(),
            model_name: String::new(),
            host: "127.0.0.1".to_string(),
            port: 11433,
            n_gpu_layers: 30,
            ctx_size: 4096,
            mtp_enabled: false,
            flash_attn: false,
        }
    }
}

fn config_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    exe.parent()
        .unwrap_or(&PathBuf::from("."))
        .join("llamacpp_config.json")
}

pub fn load_config() -> Config {
    let path = config_path();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(mut cfg) = serde_json::from_str::<Config>(&data) {
                let defaults = Config::default();
                if cfg.host.is_empty() {
                    cfg.host = defaults.host;
                }
                if cfg.port == 0 {
                    cfg.port = defaults.port;
                }
                if cfg.ctx_size == 0 {
                    cfg.ctx_size = defaults.ctx_size;
                }
                return cfg;
            }
        }
    }
    Config::default()
}

pub fn save_config(config: &Config) -> bool {
    let path = config_path();
    match serde_json::to_string_pretty(config) {
        Ok(data) => fs::write(path, data).is_ok(),
        Err(_) => false,
    }
}

pub fn list_models(model_dir: &str) -> Vec<String> {
    let mut models = Vec::new();
    if let Ok(entries) = fs::read_dir(model_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".gguf") {
                models.push(name.trim_end_matches(".gguf").to_string());
            }
        }
    }
    models.sort();
    models
}

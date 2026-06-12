use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub spec_draft_n_max: u32,
    pub flash_attn: String,
    pub command_text: String,
    pub mmproj: String,
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
            spec_draft_n_max: 2,
            flash_attn: "auto".to_string(),
            command_text: String::new(),
            mmproj: String::new(),
        }
    }
}

pub type Profiles = HashMap<String, Config>;

const META_KEY: &str = "__meta__";

fn config_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    exe.parent()
        .unwrap_or(&PathBuf::from("."))
        .join("llamacpp_config.json")
}

pub fn load_profiles() -> (Profiles, String) {
    let path = config_path();
    let mut profiles = Profiles::new();
    let mut last_profile = String::new();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(map) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(obj) = map.as_object() {
                    for (key, val) in obj {
                        if key == META_KEY {
                            if let Some(last) = val.get("last_profile").and_then(|v| v.as_str()) {
                                last_profile = last.to_string();
                            }
                        } else if let Ok(cfg) = serde_json::from_value::<Config>(val.clone()) {
                            profiles.insert(key.clone(), cfg);
                        }
                    }
                }
            }
        }
    }
    if !profiles.contains_key(&last_profile) {
        last_profile = String::new();
    }
    (profiles, last_profile)
}

pub fn save_profile(name: &str, config: &Config, last_profile: &str) -> bool {
    let path = config_path();
    let mut data = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if let Some(obj) = data.as_object_mut() {
        if let Ok(val) = serde_json::to_value(config) {
            obj.insert(name.to_string(), val);
        }
        let mut meta = obj
            .get(META_KEY)
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        meta.insert("last_profile".to_string(), serde_json::json!(last_profile));
        obj.insert(META_KEY.to_string(), serde_json::json!(meta));
    }
    fs::write(path, serde_json::to_string_pretty(&data).unwrap_or_default()).is_ok()
}

pub fn delete_profile(name: &str) -> bool {
    let path = config_path();
    if !path.exists() {
        return false;
    }
    let mut data = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or(serde_json::json!({}));
    if let Some(obj) = data.as_object_mut() {
        obj.remove(name);
        if let Some(meta) = obj.get_mut(META_KEY).and_then(|v| v.as_object_mut()) {
            if meta.get("last_profile").and_then(|v| v.as_str()) == Some(name) {
                meta.insert("last_profile".to_string(), serde_json::json!(""));
            }
        }
    }
    fs::write(path, serde_json::to_string_pretty(&data).unwrap_or_default()).is_ok()
}

pub fn list_models(model_dir: &str) -> Vec<String> {
    let mut models = Vec::new();
    if let Ok(entries) = fs::read_dir(model_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".gguf") && !name.starts_with("mmproj") {
                models.push(name.trim_end_matches(".gguf").to_string());
            }
        }
    }
    models.sort();
    models
}

pub fn list_mmproj_models(model_dir: &str) -> Vec<String> {
    let mut models = Vec::new();
    if let Ok(entries) = fs::read_dir(model_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".gguf") && name.starts_with("mmproj") {
                models.push(name.trim_end_matches(".gguf").to_string());
            }
        }
    }
    models.sort();
    models
}

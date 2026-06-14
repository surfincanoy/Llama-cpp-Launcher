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
    pub route_mode: bool,
    pub models_max: u32,
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
            route_mode: false,
            models_max: 1,
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

fn config_ini_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    exe.parent()
        .unwrap_or(&PathBuf::from("."))
        .join("config.ini")
}

pub fn config_ini_path_str() -> String {
    config_ini_path().to_string_lossy().to_string()
}

fn build_wildcard_params(config: &Config) -> HashMap<String, String> {
    let mut kvs = HashMap::new();
    kvs.insert("n-gpu-layers".to_string(), config.n_gpu_layers.to_string());
    kvs.insert("ctx-size".to_string(), config.ctx_size.to_string());
    kvs.insert("models-max".to_string(), config.models_max.to_string());
    if config.flash_attn != "auto" {
        kvs.insert("flash-attn".to_string(), config.flash_attn.clone());
    }
    if config.mtp_enabled {
        kvs.insert("spec-type".to_string(), "draft-mtp".to_string());
        kvs.insert("spec-draft-n-max".to_string(), config.spec_draft_n_max.to_string());
    }
    kvs
}

fn build_ini_section(name: &str, config: &Config, extra_args: &str) -> String {
    let model_name = if config.model_name.ends_with(".gguf") {
        config.model_name.clone()
    } else {
        format!("{}.gguf", config.model_name)
    };
    let model_path = format!("{}/{}", config.model_dir.trim_end_matches('/'), model_name);
    let mut lines = Vec::new();
    lines.push(format!("[{}]", name));
    lines.push(format!("model = {}", model_path));
    if !config.mmproj.is_empty() {
        lines.push(format!("mmproj = {}", config.mmproj));
    }
    lines.push(format!("n-gpu-layers = {}", config.n_gpu_layers));
    lines.push(format!("ctx-size = {}", config.ctx_size));
    if config.flash_attn != "auto" {
        lines.push(format!("flash-attn = {}", config.flash_attn));
    }
    if config.mtp_enabled {
        lines.push(format!("spec-type = draft-mtp"));
        lines.push(format!("spec-draft-n-max = {}", config.spec_draft_n_max));
    }
    if !extra_args.is_empty() {
        lines.push(format!("extra-args = {}", extra_args));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn parse_sections(data: &str) -> Vec<(String, HashMap<String, String>)> {
    let mut sections = Vec::new();
    let mut current_name = String::new();
    let mut current_kvs: HashMap<String, String> = HashMap::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if !current_name.is_empty() {
                sections.push((current_name.clone(), std::mem::take(&mut current_kvs)));
            }
            current_name = line[1..line.len()-1].to_string();
        } else if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let val = line[eq_pos+1..].trim().to_string();
            current_kvs.insert(key, val);
        }
    }
    if !current_name.is_empty() {
        sections.push((current_name, current_kvs));
    }
    sections
}

fn sections_to_ini(sections: &[(String, HashMap<String, String>)], with_version: bool) -> String {
    let mut lines = Vec::new();
    if with_version {
        lines.push("# version = 1".to_string());
        lines.push(String::new());
    }
    for (name, kvs) in sections {
        lines.push(format!("[{}]", name));
        let mut keys: Vec<&String> = kvs.keys().collect();
        keys.sort();
        for key in keys {
            if let Some(val) = kvs.get(key) {
                lines.push(format!("{} = {}", key, val));
            }
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

pub fn auto_generate_config_ini(config: &Config) -> bool {
    let path = config_ini_path();

    if path.exists() {
        // ensure [*] section exists
        if let Ok(data) = fs::read_to_string(&path) {
            let sections = parse_sections(&data);
            if !sections.iter().any(|(name, _)| name == "*") {
                let mut all = sections.clone();
                all.insert(0, ("*".to_string(), build_wildcard_params(config)));
                return fs::write(&path, sections_to_ini(&all, true)).is_ok();
            }
        }
        return true;
    }

    // ini does not exist — create with [*] only
    let wildcard = build_wildcard_params(config);
    let sections = vec![("*".to_string(), wildcard)];
    fs::write(&path, sections_to_ini(&sections, true)).is_ok()
}

pub fn update_config_ini_profile(profile_name: &str, config: &Config, extra_args: &str) -> bool {
    let models = list_models(&config.model_dir);
    if !models.iter().any(|m| m == profile_name) {
        return false;
    }
    let path = config_ini_path();
    let new_section = build_ini_section(profile_name, config, extra_args);

    let mut all_sections: Vec<(String, HashMap<String, String>)> = if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            parse_sections(&data)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let new_kvs = {
        let mut kvs = HashMap::new();
        for line in new_section.lines() {
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let val = line[eq_pos+1..].trim().to_string();
                kvs.insert(key, val);
            }
        }
        kvs
    };

    for (name, kvs) in &all_sections {
        if name == profile_name && *kvs == new_kvs {
            return true;
        }
    }

    let mut found = false;
    for (name, kvs) in &mut all_sections {
        if *name == profile_name {
            *kvs = new_kvs.clone();
            found = true;
            break;
        }
    }
    if !found {
        all_sections.push((profile_name.to_string(), new_kvs));
    }

    fs::write(&path, sections_to_ini(&all_sections, false)).is_ok()
}

pub fn add_config_ini_section_if_missing(profile_name: &str, config: &Config, extra_args: &str) -> bool {
    let models = list_models(&config.model_dir);
    if !models.iter().any(|m| m == profile_name) {
        return false;
    }
    let path = config_ini_path();
    if !path.exists() {
        return false;
    }
    if let Ok(data) = fs::read_to_string(&path) {
        let sections = parse_sections(&data);
        if sections.iter().any(|(name, _)| name == profile_name) {
            return true;
        }
    } else {
        return false;
    }

    // section missing — add it
    let new_section = build_ini_section(profile_name, config, extra_args);
    let mut all_sections: Vec<(String, HashMap<String, String>)> = if let Ok(data) = fs::read_to_string(&path) {
        parse_sections(&data)
    } else {
        Vec::new()
    };
    let new_kvs = {
        let mut kvs = HashMap::new();
        for line in new_section.lines() {
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let val = line[eq_pos+1..].trim().to_string();
                kvs.insert(key, val);
            }
        }
        kvs
    };
    all_sections.push((profile_name.to_string(), new_kvs));
    fs::write(&path, sections_to_ini(&all_sections, false)).is_ok()
}

pub fn sync_ini_model_sections(
    models: &[String],
    config: &Config,
    extra_args: &str,
    profiles: &HashMap<String, Config>,
) -> bool {
    let path = config_ini_path();
    if !path.exists() {
        return false;
    }
    let Ok(data) = fs::read_to_string(&path) else { return false };
    let mut sections = parse_sections(&data);
    let mut changed = false;
    for model_name in models {
        if model_name == "*" {
            continue;
        }
        let (src_cfg, src_extra, from_profile) = if let Some(profile_cfg) = profiles.get(model_name) {
            let extra = extract_extra_args(&profile_cfg.command_text);
            (profile_cfg.clone(), extra, true)
        } else {
            (config.clone(), extra_args.to_string(), false)
        };
        let model_file = format!("{}.gguf", model_name);
        let model_path = format!("{}/{}", src_cfg.model_dir.trim_end_matches('/'), model_file);
        let mut kvs = HashMap::new();
        kvs.insert("model".to_string(), model_path);
        if !src_cfg.mmproj.is_empty() {
            kvs.insert("mmproj".to_string(), src_cfg.mmproj.clone());
        }
        kvs.insert("n-gpu-layers".to_string(), src_cfg.n_gpu_layers.to_string());
        kvs.insert("ctx-size".to_string(), src_cfg.ctx_size.to_string());
        if src_cfg.flash_attn != "auto" {
            kvs.insert("flash-attn".to_string(), src_cfg.flash_attn.clone());
        }
        if src_cfg.mtp_enabled {
            kvs.insert("spec-type".to_string(), "draft-mtp".to_string());
            kvs.insert("spec-draft-n-max".to_string(), src_cfg.spec_draft_n_max.to_string());
        }
        if !src_extra.is_empty() {
            kvs.insert("extra-args".to_string(), src_extra);
        }
        if let Some(pos) = sections.iter().position(|(n, _)| n == model_name) {
            if from_profile && sections[pos].1 != kvs {
                sections[pos].1 = kvs;
                changed = true;
            }
        } else {
            sections.push((model_name.to_string(), kvs));
            changed = true;
        }
    }
    if changed {
        fs::write(&path, sections_to_ini(&sections, true)).is_ok()
    } else {
        true
    }
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

pub fn extract_extra_args(command_text: &str) -> String {
    let known_flags = ["-m", "--host", "--port", "-c", "--n-gpu-layers",
                       "--spec-type", "--spec-draft-n-max", "--flash-attn", "--mmproj",
                       "--models-preset", "--models-max"];
    let tokens: Vec<&str> = command_text.split_whitespace().collect();
    let mut extra = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if known_flags.contains(&tokens[i]) || tokens[i].starts_with("--flash-attn=") {
            if tokens[i] == "--flash-attn" {
                if let Some(next) = tokens.get(i + 1) {
                    if *next == "on" || *next == "off" || *next == "auto" {
                        i += 1;
                    }
                }
            } else {
                if let Some(_) = tokens.get(i + 1) {
                    i += 1;
                }
            }
        } else {
            if i > 0 {
                extra.push(tokens[i]);
            }
        }
        i += 1;
    }
    extra.join(" ")
}

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic::atomic_write;
use crate::error::MnemeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_decay")]
    pub decay: f64,
    #[serde(default = "default_spread")]
    pub spread_decay: f64,
    #[serde(default = "default_tokens")]
    pub default_tokens: usize,
    #[serde(default = "default_wm")]
    pub working_memory: usize,
    #[serde(default = "default_lang")]
    pub language: String,
}

fn default_decay() -> f64 {
    0.5
}
fn default_spread() -> f64 {
    0.45
}
fn default_tokens() -> usize {
    700
}
fn default_wm() -> usize {
    5
}
fn default_lang() -> String {
    "en".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            decay: default_decay(),
            spread_decay: default_spread(),
            default_tokens: default_tokens(),
            working_memory: default_wm(),
            language: default_lang(),
        }
    }
}

pub fn default_vault_path() -> PathBuf {
    if let Ok(v) = env::var("MNEME_VAULT") {
        if !v.trim().is_empty() {
            return PathBuf::from(v);
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Dropbox")
        .join("md")
}

pub fn load_or_create(mneme_dir: &Path) -> Result<Config, MnemeError> {
    fs::create_dir_all(mneme_dir)?;
    let path = mneme_dir.join("config.toml");
    if path.exists() {
        let raw = fs::read_to_string(&path)?;
        let cfg: Config = toml_from_str(&raw)?;
        return Ok(cfg);
    }
    let cfg = Config::default();
    let body = format!(
        "decay = {}\nspread_decay = {}\ndefault_tokens = {}\nworking_memory = {}\nlanguage = \"en\"\n",
        cfg.decay, cfg.spread_decay, cfg.default_tokens, cfg.working_memory
    );
    atomic_write(&path, body.as_bytes())?;
    Ok(cfg)
}

fn toml_from_str(raw: &str) -> Result<Config, MnemeError> {
    let mut cfg = Config::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim().trim_matches('"');
        match k {
            "decay" => cfg.decay = v.parse().unwrap_or(cfg.decay),
            "spread_decay" => cfg.spread_decay = v.parse().unwrap_or(cfg.spread_decay),
            "default_tokens" => cfg.default_tokens = v.parse().unwrap_or(cfg.default_tokens),
            "working_memory" => cfg.working_memory = v.parse().unwrap_or(cfg.working_memory),
            "language" => cfg.language = v.to_string(),
            _ => {}
        }
    }
    Ok(cfg)
}

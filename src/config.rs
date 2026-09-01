use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;
use crate::error::MnemeError;

#[derive(Debug, Clone)]
pub struct Config {
    pub decay: f64,
    pub spread_decay: f64,
    pub default_tokens: usize,
    pub working_memory: usize,
    pub language: String,
    pub vault: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            decay: 0.5,
            spread_decay: 0.45,
            default_tokens: 700,
            working_memory: 5,
            language: "en".into(),
            vault: None,
        }
    }
}

/// Resolve the vault directory without assuming a personal machine layout.
///
/// Order: `MNEME_VAULT` → `vault` in the user config file → current dir if it
/// looks like a vault. Callers still honor an explicit `--vault` flag first.
pub fn default_vault_path() -> PathBuf {
    if let Ok(v) = env::var("MNEME_VAULT") {
        if !v.trim().is_empty() {
            return PathBuf::from(v);
        }
    }
    if let Some(cfg) = load_user_config() {
        if let Some(vault) = cfg.vault {
            return vault;
        }
    }
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if looks_like_vault(&cwd) {
        return cwd;
    }
    cwd
}

pub fn looks_like_vault(dir: &Path) -> bool {
    dir.join("INDEX.md").is_file() || dir.join("AGENTS.md").is_file()
}

pub fn user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("mneme").join("config.toml"))
}

pub fn load_user_config() -> Option<Config> {
    let path = user_config_path()?;
    let raw = fs::read_to_string(path).ok()?;
    Some(parse_toml(&raw))
}

pub fn load_or_create(mneme_dir: &Path) -> Result<Config, MnemeError> {
    fs::create_dir_all(mneme_dir)?;
    let path = mneme_dir.join("config.toml");
    if path.exists() {
        let raw = fs::read_to_string(&path)?;
        return Ok(parse_toml(&raw));
    }
    let cfg = Config::default();
    let body = format!(
        "# Vault-local settings. Do not put machine-specific home paths in git.\ndecay = {}\nspread_decay = {}\ndefault_tokens = {}\nworking_memory = {}\nlanguage = \"en\"\n",
        cfg.decay, cfg.spread_decay, cfg.default_tokens, cfg.working_memory
    );
    atomic_write(&path, body.as_bytes())?;
    Ok(cfg)
}

fn parse_toml(raw: &str) -> Config {
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
            "vault" => {
                if !v.is_empty() {
                    cfg.vault = Some(PathBuf::from(v));
                }
            }
            _ => {}
        }
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ignores_personal_comments() {
        let cfg = parse_toml("language = \"en\"\n# vault = \"/home/someone/secret\"\n");
        assert!(cfg.vault.is_none());
        assert_eq!(cfg.language, "en");
    }

    #[test]
    fn looks_like_vault_needs_index_or_agents() {
        assert!(!looks_like_vault(Path::new("/this/does/not/exist-mneme")));
    }
}

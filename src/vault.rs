use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use walkdir::WalkDir;

use crate::atomic::atomic_write;
use crate::config::{self, Config};
use crate::error::MnemeError;
use crate::journal::Journal;
use crate::note::Note;

const SKIP: &[&str] = &[
    ".obsidian",
    ".mneme",
    "templates",
    "graph",
    "grafo",
    "99-attachments",
    "99-anexos",
    "00-system/scripts",
    "00-sistema",
];

pub struct Vault {
    pub root: PathBuf,
    pub config: Config,
    pub journal: Journal,
    lock_file: File,
}

impl Vault {
    pub fn open(root: PathBuf) -> Result<Self, MnemeError> {
        if !root.is_dir() {
            return Err(MnemeError::VaultMissing(root));
        }
        let mneme_dir = root.join(".mneme");
        fs::create_dir_all(&mneme_dir)?;
        fs::create_dir_all(mneme_dir.join("tmp"))?;
        let lock_path = mneme_dir.join("lock");
        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;
        if lock_file.try_lock_exclusive().is_err() {
            return Err(MnemeError::Locked);
        }
        let config = config::load_or_create(&mneme_dir)?;
        let journal = Journal::open(&mneme_dir)?;
        Ok(Self {
            root,
            config,
            journal,
            lock_file,
        })
    }

    pub fn mneme_dir(&self) -> PathBuf {
        self.root.join(".mneme")
    }

    pub fn db_path(&self) -> PathBuf {
        self.mneme_dir().join("index.sqlite")
    }

    pub fn iter_notes(&self) -> Result<Vec<Note>, MnemeError> {
        let mut notes = Vec::new();
        for entry in WalkDir::new(&self.root).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let rel = path.strip_prefix(&self.root).unwrap_or(path);
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            if should_skip(&rel_s) {
                continue;
            }
            let raw = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            match Note::parse(&rel_s, &raw) {
                Ok(n) => notes.push(n),
                Err(err) => tracing::warn!("skip unreadable note {rel_s}: {err}"),
            }
        }
        notes.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(notes)
    }

    pub fn write_note(&self, note: &Note) -> Result<(), MnemeError> {
        let path = self.root.join(&note.rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let rendered = note.render()?;
        atomic_write(&path, rendered.as_bytes())
    }

    pub fn abs(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }
}

impl Drop for Vault {
    fn drop(&mut self) {
        let _ = self.lock_file.unlock();
    }
}

fn should_skip(rel: &str) -> bool {
    SKIP.iter().any(|p| rel == *p || rel.starts_with(&format!("{p}/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_obsidian_and_templates() {
        assert!(should_skip("templates/note.md"));
        assert!(should_skip(".obsidian/app.json"));
        assert!(should_skip(".mneme/index.sqlite"));
        assert!(!should_skip("02-memory/Facts.md"));
    }
}

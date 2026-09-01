use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic::sync_dir;
use crate::error::MnemeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEvent {
    pub id: String,
    pub ts: i64,
    pub op: String,
    pub path: String,
    pub hash: String,
    pub phase: String,
}

pub struct Journal {
    path: PathBuf,
}

impl Journal {
    pub fn open(mneme_dir: &Path) -> Result<Self, MnemeError> {
        fs::create_dir_all(mneme_dir)?;
        Ok(Self {
            path: mneme_dir.join("journal.jsonl"),
        })
    }

    pub fn append(&self, event: &JournalEvent) -> Result<(), MnemeError> {
        let line = serde_json::to_string(event)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        if let Some(dir) = self.path.parent() {
            sync_dir(dir)?;
        }
        Ok(())
    }

    pub fn tail(&self, n: usize) -> Result<Vec<JournalEvent>, MnemeError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&self.path)?;
        let mut rows = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(ev) = serde_json::from_str::<JournalEvent>(&line) {
                rows.push(ev);
            }
        }
        if rows.len() > n {
            rows.drain(0..rows.len() - n);
        }
        Ok(rows)
    }
}

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

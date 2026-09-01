use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::compact::estimate_tokens;
use crate::config::Config;
use crate::error::MnemeError;
use crate::journal::now_ts;
use crate::note::Note;
use crate::vault::Vault;

pub struct Index {
    pub conn: Connection,
}

impl Index {
    pub fn open(path: &Path) -> Result<Self, MnemeError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            "#,
        )?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS notes (
              id TEXT PRIMARY KEY,
              path TEXT NOT NULL UNIQUE,
              kind TEXT NOT NULL,
              status TEXT NOT NULL DEFAULT 'active',
              title TEXT NOT NULL,
              body_hash TEXT NOT NULL,
              word_count INTEGER NOT NULL,
              token_est INTEGER NOT NULL,
              summary TEXT NOT NULL,
              updated_ts INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS links (
              src TEXT NOT NULL,
              dst TEXT NOT NULL,
              PRIMARY KEY (src, dst)
            );
            CREATE TABLE IF NOT EXISTS accesses (
              note_id TEXT NOT NULL,
              ts INTEGER NOT NULL,
              kind TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS activation (
              note_id TEXT PRIMARY KEY,
              base_level REAL NOT NULL DEFAULT 0,
              last_ts INTEGER NOT NULL
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
              id, title, summary, body, kind, tags,
              tokenize = 'unicode61'
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn rebuild(&self, vault: &Vault) -> Result<usize, MnemeError> {
        let notes = vault.iter_notes()?;
        self.conn.execute_batch("BEGIN IMMEDIATE;")?;
        let tx = || -> Result<usize, MnemeError> {
            self.conn.execute("DELETE FROM notes", [])?;
            self.conn.execute("DELETE FROM links", [])?;
            self.conn.execute("DELETE FROM notes_fts", [])?;
            let mut n = 0;
            for note in &notes {
                self.upsert_note(note)?;
                n += 1;
            }
            Ok(n)
        };
        match tx() {
            Ok(n) => {
                self.conn.execute_batch("COMMIT;")?;
                self.refresh_activation(&vault.config)?;
                Ok(n)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK;");
                Err(e)
            }
        }
    }

    pub fn upsert_note(&self, note: &Note) -> Result<(), MnemeError> {
        let hash = sha256(note.raw.as_bytes());
        let words = note.body.split_whitespace().count() as i64;
        let summary = note.summary(280);
        let token_est = estimate_tokens(&format!("{} {}", note.title(), note.body)) as i64;
        let updated_ts = parse_updated(&note.front.updated).unwrap_or_else(now_ts);
        let kind = note.kind().to_string();
        let status = if note.front.status.is_empty() {
            "active".into()
        } else {
            note.front.status.clone()
        };
        let tags = note.front.tags.join(" ");
        self.conn.execute(
            r#"
            INSERT INTO notes (id, path, kind, status, title, body_hash, word_count, token_est, summary, updated_ts)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
              path=excluded.path, kind=excluded.kind, status=excluded.status, title=excluded.title,
              body_hash=excluded.body_hash, word_count=excluded.word_count, token_est=excluded.token_est,
              summary=excluded.summary, updated_ts=excluded.updated_ts
            "#,
            params![
                note.id,
                note.rel_path,
                kind,
                status,
                note.title(),
                hash,
                words,
                token_est,
                summary,
                updated_ts
            ],
        )?;
        self.conn
            .execute("DELETE FROM links WHERE src = ?1", params![note.id])?;
        for dst in note.wikilinks() {
            self.conn.execute(
                "INSERT OR IGNORE INTO links (src, dst) VALUES (?1, ?2)",
                params![note.id, dst],
            )?;
        }
        self.conn
            .execute("DELETE FROM notes_fts WHERE id = ?1", params![note.id])?;
        self.conn.execute(
            "INSERT INTO notes_fts (id, title, summary, body, kind, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![note.id, note.title(), summary, note.body, kind, tags],
        )?;
        Ok(())
    }

    pub fn record_access(&self, note_id: &str, kind: &str) -> Result<(), MnemeError> {
        self.conn.execute(
            "INSERT INTO accesses (note_id, ts, kind) VALUES (?1, ?2, ?3)",
            params![note_id, now_ts(), kind],
        )?;
        Ok(())
    }

    pub fn refresh_activation(&self, cfg: &Config) -> Result<(), MnemeError> {
        let ids: Vec<String> = {
            let mut stmt = self.conn.prepare("SELECT id FROM notes")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let now = now_ts() as f64;
        let d = cfg.decay;
        for id in ids {
            let times: Vec<i64> = {
                let mut stmt = self
                    .conn
                    .prepare("SELECT ts FROM accesses WHERE note_id = ?1 ORDER BY ts DESC LIMIT 24")?;
                let rows = stmt.query_map(params![id], |r| r.get::<_, i64>(0))?;
                rows.filter_map(|r| r.ok()).collect()
            };
            let mut times = times;
            if times.is_empty() {
                if let Some(ts) = self
                    .conn
                    .query_row(
                        "SELECT updated_ts FROM notes WHERE id = ?1",
                        params![id],
                        |r| r.get::<_, i64>(0),
                    )
                    .optional()?
                {
                    times.push(ts);
                }
            }
            let mut sum = 0.0;
            for ts in &times {
                let age = (now - *ts as f64).max(1.0);
                sum += age.powf(-d);
            }
            let base = sum.max(1e-9).ln();
            let last = times.first().copied().unwrap_or(now_ts());
            self.conn.execute(
                r#"
                INSERT INTO activation (note_id, base_level, last_ts)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(note_id) DO UPDATE SET base_level=excluded.base_level, last_ts=excluded.last_ts
                "#,
                params![id, base, last],
            )?;
        }
        Ok(())
    }

    pub fn neighbors(&self) -> Result<HashMap<String, Vec<String>>, MnemeError> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT src, dst FROM links")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows.flatten() {
            map.entry(row.0.clone()).or_default().push(row.1.clone());
            map.entry(row.1).or_default().push(row.0);
        }
        Ok(map)
    }

    pub fn cards(&self) -> Result<Vec<Card>, MnemeError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT n.id, n.path, n.kind, n.status, n.title, n.summary, n.token_est,
                   COALESCE(a.base_level, 0)
            FROM notes n
            LEFT JOIN activation a ON a.note_id = n.id
            "#,
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Card {
                id: r.get(0)?,
                path: r.get(1)?,
                kind: r.get(2)?,
                status: r.get(3)?,
                title: r.get(4)?,
                summary: r.get(5)?,
                token_est: r.get::<_, i64>(6)? as usize,
                activation: r.get(7)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn fts_search(&self, query: &str) -> Result<Vec<(String, f64)>, MnemeError> {
        let q = fts_query(query);
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, bm25(notes_fts) FROM notes_fts WHERE notes_fts MATCH ?1 ORDER BY bm25(notes_fts) LIMIT 40",
        )?;
        let rows = match stmt.query_map(params![q], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
        }) {
            Ok(rows) => rows,
            Err(_) => return Ok(Vec::new()),
        };
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn links_from(&self, id: &str) -> Result<Vec<String>, MnemeError> {
        let mut stmt = self
            .conn
            .prepare("SELECT dst FROM links WHERE src = ?1")?;
        let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn vacuum(&self) -> Result<(), MnemeError> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Card {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub status: String,
    pub title: String,
    pub summary: String,
    pub token_est: usize,
    pub activation: f64,
}

fn fts_query(raw: &str) -> String {
    const STOP: &[&str] = &[
        "a", "an", "the", "and", "or", "of", "to", "in", "on", "for", "is", "are", "be",
    ];
    raw.split_whitespace()
        .filter(|t| t.len() > 1 && !STOP.iter().any(|s| s.eq_ignore_ascii_case(t)))
        .map(|t| {
            t.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>()
        })
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(" OR ")
}

pub fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

fn parse_updated(s: &str) -> Option<i64> {
    let s = s.trim();
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_r_is_finite() {
        let age = 3600.0_f64;
        let sum = age.powf(-0.5);
        assert!(sum.ln().is_finite());
    }
}

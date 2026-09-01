use std::fs;

use walkdir::WalkDir;

use crate::error::MnemeError;
use crate::index::Index;
use crate::vault::Vault;

pub fn run(vault: &Vault) -> Result<String, MnemeError> {
    let mut tmp = 0usize;
    let mut conflicts = Vec::new();
    for entry in WalkDir::new(&vault.root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if name.contains(".mneme.tmp") {
            let _ = fs::remove_file(entry.path());
            tmp += 1;
        }
        if name.to_lowercase().contains("conflicted copy") {
            conflicts.push(entry.path().display().to_string());
        }
    }
    let db = vault.db_path();
    if db.exists() {
        let ping = rusqlite::Connection::open(&db).and_then(|c| {
            c.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))
        });
        if ping.is_err() {
            let _ = fs::remove_file(&db);
            let _ = fs::remove_file(db.with_extension("sqlite-wal"));
            let _ = fs::remove_file(db.with_extension("sqlite-shm"));
        }
    }
    let index = Index::open(&vault.db_path())?;
    let n = index.rebuild(vault)?;
    let (nodes, edges) = crate::graph::export(vault)?;
    Ok(format!(
        "doctor: rebuilt {n} notes, graph {nodes}/{edges}, removed {tmp} tmp files, conflict copies: {}",
        if conflicts.is_empty() {
            "none".into()
        } else {
            conflicts.join(", ")
        }
    ))
}

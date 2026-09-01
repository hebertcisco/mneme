use crate::error::MnemeError;
use crate::index::sha256;
use crate::journal::{now_ts, JournalEvent};
use crate::note::{folder_for_kind, hub_for_kind, slug_title, FrontMatter, Note};
use crate::vault::Vault;

pub struct RememberOpts {
    pub kind: String,
    pub title: String,
    pub body: String,
}

pub fn remember(vault: &Vault, index: &crate::index::Index, opts: RememberOpts) -> Result<String, MnemeError> {
    let kind = if opts.kind.is_empty() {
        "memory".into()
    } else {
        opts.kind
    };
    let title = opts.title.trim();
    if title.is_empty() {
        return Err(MnemeError::other("remember needs --title"));
    }
    let slug = slug_title(title);
    let folder = folder_for_kind(&kind);
    let rel = format!("{folder}/{slug}.md");
    let dest = vault.root.join(&rel);
    if dest.exists() {
        return Err(MnemeError::other(format!(
            "note already exists: {rel} (use a different --title)"
        )));
    }
    let hub = hub_for_kind(&kind);
    let mut body = opts.body.trim().to_string();
    if body.is_empty() {
        body = format!("Related: [[{hub}]]\n");
    } else if !body.contains(&format!("[[{hub}]]")) {
        body.push_str(&format!("\n\nRelated: [[{hub}]]\n"));
    }
    if !body.contains("[[INDEX]]") && hub != "INDEX" {
        body.push_str("\nSee [[INDEX]].\n");
    }
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let note = Note {
        id: slug.clone(),
        rel_path: rel.clone(),
        front: FrontMatter {
            kind: kind.clone(),
            status: "active".into(),
            tags: Vec::new(),
            updated: today,
            extra: Default::default(),
        },
        body: format!("# {title}\n\n{body}"),
        raw: String::new(),
    };
    let rendered = note.render()?;
    vault.journal.append(&JournalEvent {
        id: uuid::Uuid::new_v4().to_string(),
        ts: now_ts(),
        op: "remember".into(),
        path: rel.clone(),
        hash: sha256(rendered.as_bytes()),
        phase: "commit".into(),
    })?;
    vault.write_note(&note)?;
    let stored = Note::parse(&rel, &fs_read(&vault.root.join(&rel))?)?;
    index.upsert_note(&stored)?;
    index.record_access(&stored.id, "encode")?;
    index.refresh_activation(&vault.config)?;
    let _ = crate::graph::export(vault);
    Ok(rel)
}

fn fs_read(path: &std::path::Path) -> Result<String, MnemeError> {
    Ok(std::fs::read_to_string(path)?)
}

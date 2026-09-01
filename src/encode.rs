use crate::error::MnemeError;
use crate::index::sha256;
use crate::journal::{now_ts, JournalEvent};
use crate::note::{folder_for_kind, hub_for_kind, slug_title, FrontMatter, Note};
use crate::vault::Vault;

pub struct RememberOpts {
    pub kind: String,
    pub title: String,
    pub body: String,
    /// Explicit ISO 639-1 tag. When none, detect from title+body (no translation).
    pub lang: Option<String>,
}

pub fn remember(
    vault: &Vault,
    index: &crate::index::Index,
    opts: RememberOpts,
) -> Result<String, MnemeError> {
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
    let sample = format!("{title}\n{body}");
    let lang = match opts.lang.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(tag) => crate::lang::normalize(tag),
        None => crate::lang::detect(&sample, &vault.config.language),
    };
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let note = Note {
        id: slug.clone(),
        rel_path: rel.clone(),
        front: FrontMatter {
            kind: kind.clone(),
            status: "active".into(),
            tags: Vec::new(),
            updated: today,
            lang,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Index;

    fn fixture() -> (tempfile::TempDir, Vault, Index) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("INDEX.md"), "# Index\n").unwrap();
        let vault = Vault::open(dir.path().to_path_buf()).unwrap();
        let index = Index::open(&vault.db_path()).unwrap();
        (dir, vault, index)
    }

    #[test]
    fn remember_keeps_portuguese_and_sets_lang() {
        let (_dir, vault, index) = fixture();
        let rel = remember(
            &vault,
            &index,
            RememberOpts {
                kind: "memory".into(),
                title: "Citacao original".into(),
                body: "Não traduza este parágrafo; grave no idioma original com metadata de língua."
                    .into(),
                lang: None,
            },
        )
        .unwrap();
        let raw = std::fs::read_to_string(vault.root.join(&rel)).unwrap();
        assert!(raw.contains("lang: pt"), "frontmatter should tag Portuguese:\n{raw}");
        assert!(raw.contains("Não traduza este parágrafo"));
        assert!(!raw.to_lowercase().contains("do not translate"));
    }

    #[test]
    fn remember_explicit_lang_wins_and_body_stays() {
        let (_dir, vault, index) = fixture();
        let body = "The vault index can be rebuilt from markdown notes without translation.";
        let rel = remember(
            &vault,
            &index,
            RememberOpts {
                kind: "memory".into(),
                title: "Quoted source".into(),
                body: body.into(),
                lang: Some("pt-BR".into()),
            },
        )
        .unwrap();
        let raw = std::fs::read_to_string(vault.root.join(&rel)).unwrap();
        assert!(raw.contains("lang: pt"));
        assert!(raw.contains(body));
    }
}

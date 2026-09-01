use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::config;
use crate::encode::RememberOpts;
use crate::index::Index;
use crate::recall::RecallOpts;
use crate::vault::Vault;

#[derive(Parser)]
#[command(name = "mneme", version, about = "Crash-safe Markdown memory engine")]
struct Args {
    /// Vault directory (overrides MNEME_VAULT and user config)
    #[arg(long, global = true, env = "MNEME_VAULT")]
    vault: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create `.mneme/` and index existing notes
    Init,
    /// Rebuild the SQLite index from Markdown
    Reindex,
    /// Ranked compact retrieval
    Recall {
        query: String,
        #[arg(long, default_value_t = 700)]
        tokens: usize,
        #[arg(long, default_value = "ai")]
        format: String,
    },
    /// Write a new note. Body is stored in the source language; `lang` is tagged.
    Remember {
        #[arg(long, default_value = "memory")]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: Option<String>,
        /// ISO 639-1 tag (e.g. pt). Detected from the body when omitted.
        #[arg(long)]
        lang: Option<String>,
    },
    /// Export wikilink graph
    Graph,
    /// Refresh activation, graph, and vacuum
    Consolidate,
    /// Repair tmp files, conflicts, and the index
    Doctor,
    /// Vault and index summary
    Status,
    /// Show one note by id (file stem)
    Show { id: String },
    /// Write `.mneme/context.md` for agents
    Context {
        #[arg(long, default_value_t = 700)]
        tokens: usize,
    },
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    let root = match args.vault {
        Some(p) => p,
        None => config::resolve_vault_path()?,
    };
    match args.cmd {
        Command::Init => {
            let vault = open_vault(&root)?;
            let index = Index::open(&vault.db_path())?;
            let n = index.rebuild(&vault)?;
            let (nodes, edges) = crate::graph::export(&vault)?;
            println!(
                "initialized {} notes, graph {nodes}/{edges} at {}",
                n,
                vault.root.display()
            );
        }
        Command::Reindex => {
            let vault = open_vault(&root)?;
            let index = Index::open(&vault.db_path())?;
            let n = index.rebuild(&vault)?;
            println!("reindexed {n} notes");
        }
        Command::Recall {
            query,
            tokens,
            format,
        } => {
            let vault = open_vault(&root)?;
            let index = Index::open(&vault.db_path())?;
            if index.cards()?.is_empty() {
                index.rebuild(&vault)?;
            }
            let out = crate::recall::recall(
                &vault,
                &index,
                &RecallOpts {
                    query,
                    tokens,
                    format,
                },
            )?;
            print!("{out}");
        }
        Command::Remember {
            kind,
            title,
            body,
            lang,
        } => {
            let vault = open_vault(&root)?;
            let index = Index::open(&vault.db_path())?;
            let body = match body {
                Some(b) => b,
                None => {
                    if atty_stdin() {
                        String::new()
                    } else {
                        let mut buf = String::new();
                        io::stdin().read_to_string(&mut buf)?;
                        buf
                    }
                }
            };
            let rel = crate::encode::remember(
                &vault,
                &index,
                RememberOpts {
                    kind,
                    title,
                    body,
                    lang,
                },
            )?;
            println!("wrote {rel}");
        }
        Command::Graph => {
            let vault = open_vault(&root)?;
            let (nodes, edges) = crate::graph::export(&vault)?;
            println!("graph: {nodes} nodes, {edges} edges");
        }
        Command::Consolidate => {
            let vault = open_vault(&root)?;
            let index = Index::open(&vault.db_path())?;
            println!("{}", crate::consolidate::run(&vault, &index)?);
        }
        Command::Doctor => {
            let vault = open_vault(&root)?;
            println!("{}", crate::doctor::run(&vault)?);
        }
        Command::Status => {
            let vault = open_vault(&root)?;
            let notes = vault.iter_notes()?.len();
            println!("vault:  {}", vault.root.display());
            println!("index:  {}", vault.db_path().display());
            println!("notes:  {notes}");
            println!("lang:   {}", vault.config.language);
        }
        Command::Show { id } => {
            let vault = open_vault(&root)?;
            let notes = vault.iter_notes()?;
            let Some(note) = notes.iter().find(|n| n.id.eq_ignore_ascii_case(&id)) else {
                anyhow::bail!("note not found: {id}");
            };
            println!("{} ({})\n", note.rel_path, note.kind());
            print!("{}", note.raw);
        }
        Command::Context { tokens } => {
            let vault = open_vault(&root)?;
            let index = Index::open(&vault.db_path())?;
            if index.cards()?.is_empty() {
                index.rebuild(&vault)?;
            }
            let out = crate::recall::recall(
                &vault,
                &index,
                &RecallOpts {
                    query: "INDEX Facts AGENTS".into(),
                    tokens,
                    format: "ai".into(),
                },
            )?;
            let path = vault.mneme_dir().join("context.md");
            crate::atomic::atomic_write(&path, out.as_bytes())?;
            println!("wrote {}", path.display());
        }
    }
    Ok(())
}

fn open_vault(root: &PathBuf) -> Result<Vault> {
    Vault::open(root.clone()).with_context(|| format!("open vault {}", root.display()))
}

fn atty_stdin() -> bool {
    std::io::IsTerminal::is_terminal(&io::stdin())
}

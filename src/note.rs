use std::collections::BTreeMap;
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::MnemeError;

const WIKILINK_RE: &str = r"\[\[([^\]|#]+)(?:[#|][^\]]*)?\]\]";
const IGNORE: &[&str] = &["wikilink", "note", "slug", "title", "name"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrontMatter {
    #[serde(default)]
    pub kind: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub updated: String,
    /// ISO 639-1 (or similar). Empty means unspecified; treat as vault default.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lang: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

fn default_status() -> String {
    "active".into()
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub rel_path: String,
    pub front: FrontMatter,
    pub body: String,
    pub raw: String,
}

impl Note {
    pub fn parse(rel_path: &str, raw: &str) -> Result<Self, MnemeError> {
        let id = Path::new(rel_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("note")
            .to_string();
        let (front, body) = split_frontmatter(raw)?;
        Ok(Self {
            id,
            rel_path: rel_path.replace('\\', "/"),
            front,
            body,
            raw: raw.to_string(),
        })
    }

    pub fn render(&self) -> Result<String, MnemeError> {
        let yaml = serde_yaml::to_string(&self.front)?;
        let yaml = yaml.trim_end();
        let body = if self.body.starts_with('\n') {
            self.body.clone()
        } else {
            format!("\n{}", self.body)
        };
        Ok(format!("---\n{yaml}\n---\n{body}"))
    }

    pub fn wikilinks(&self) -> Vec<String> {
        extract_wikilinks(&self.raw)
    }

    pub fn title(&self) -> String {
        self.body
            .lines()
            .find_map(|l| l.strip_prefix("# ").map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.id.clone())
    }

    pub fn summary(&self, max_chars: usize) -> String {
        let mut skip_heading = true;
        let mut buf = String::new();
        for line in self.body.lines() {
            let t = line.trim();
            if skip_heading && t.starts_with('#') {
                skip_heading = false;
                continue;
            }
            skip_heading = false;
            if t.is_empty() {
                if !buf.is_empty() {
                    break;
                }
                continue;
            }
            if !buf.is_empty() {
                buf.push(' ');
            }
            buf.push_str(t);
            if buf.len() >= max_chars {
                break;
            }
        }
        if buf.len() > max_chars {
            buf.truncate(max_chars);
            buf.push('…');
        }
        buf
    }

    pub fn kind(&self) -> &str {
        if self.front.kind.is_empty() {
            "note"
        } else {
            &self.front.kind
        }
    }

    /// Language tag for this note. Empty frontmatter → `fallback` (vault default).
    pub fn lang_or(&self, fallback: &str) -> String {
        crate::lang::resolve(&self.front.lang, fallback)
    }
}

pub fn split_frontmatter(raw: &str) -> Result<(FrontMatter, String), MnemeError> {
    let text = raw.trim_start_matches('\u{feff}');
    if !text.starts_with("---") {
        return Ok((FrontMatter::default(), text.to_string()));
    }
    let rest = &text[3..];
    let rest = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"))
        .unwrap_or(rest);
    let Some(end) = rest.find("\n---") else {
        return Ok((FrontMatter::default(), text.to_string()));
    };
    let yaml = &rest[..end];
    let after = &rest[end + 4..];
    let body = after
        .strip_prefix('\n')
        .or_else(|| after.strip_prefix("\r\n"))
        .unwrap_or(after)
        .to_string();
    let front: FrontMatter = if yaml.trim().is_empty() {
        FrontMatter::default()
    } else {
        serde_yaml::from_str(yaml)?
    };
    Ok((front, body))
}

pub fn extract_wikilinks(text: &str) -> Vec<String> {
    let re = Regex::new(WIKILINK_RE).expect("wikilink regex");
    let mut out = Vec::new();
    for cap in re.captures_iter(text) {
        let name = cap[1].trim();
        if name.is_empty() {
            continue;
        }
        if IGNORE.iter().any(|i| i.eq_ignore_ascii_case(name)) {
            continue;
        }
        if !out.iter().any(|e: &String| e == name) {
            out.push(name.to_string());
        }
    }
    out
}

pub fn folder_for_kind(kind: &str) -> &'static str {
    match kind {
        "system" => "00-system",
        "identity" => "01-identity",
        "memory" => "02-memory",
        "project" => "03-projects",
        "entity" => "04-entities",
        "journal" => "05-journal",
        "sdd" => "06-sdd",
        "agent" => "07-agents",
        _ => "02-memory",
    }
}

pub fn hub_for_kind(kind: &str) -> &'static str {
    match kind {
        "memory" => "Facts",
        "identity" => "INDEX",
        "project" => "Projects",
        "entity" => "Entities",
        "journal" => "INDEX",
        "sdd" => "SDD",
        "agent" => "Agents",
        "system" => "INDEX",
        _ => "INDEX",
    }
}

pub fn slug_title(title: &str) -> String {
    let mut s: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                ' '
            }
        })
        .collect();
    while s.contains("  ") {
        s = s.replace("  ", " ");
    }
    let s = s.trim().replace(' ', "-");
    if s.is_empty() {
        "note".into()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_links() {
        let raw = "---\nkind: memory\nstatus: active\ntags:\n  - fact\nupdated: 2026-08-31\n---\n# Hello\n\nSee [[Person]] and [[Facts]].\n";
        let note = Note::parse("02-memory/Hello.md", raw).unwrap();
        assert_eq!(note.front.kind, "memory");
        assert_eq!(note.title(), "Hello");
        assert_eq!(
            note.wikilinks(),
            vec!["Person".to_string(), "Facts".to_string()]
        );
        assert!(note.summary(80).contains("See"));
    }

    #[test]
    fn roundtrip_keeps_kind() {
        let raw = "---\nkind: identity\nstatus: active\ntags: []\nupdated: 2026-08-31\n---\n# Person\n\nOwner.\n";
        let mut note = Note::parse("01-identity/Person.md", raw).unwrap();
        note.front.updated = "2026-09-01".into();
        let rendered = note.render().unwrap();
        let again = Note::parse("01-identity/Person.md", &rendered).unwrap();
        assert_eq!(again.front.kind, "identity");
        assert_eq!(again.front.updated, "2026-09-01");
        assert!(again.body.contains("Owner"));
    }

    #[test]
    fn roundtrip_keeps_lang() {
        let raw = "---\nkind: memory\nstatus: active\ntags: []\nupdated: 2026-09-01\nlang: pt\n---\n# Citacao\n\nNão traduza.\n";
        let note = Note::parse("02-memory/Citacao.md", raw).unwrap();
        assert_eq!(note.front.lang, "pt");
        assert_eq!(note.lang_or("en"), "pt");
        let rendered = note.render().unwrap();
        assert!(rendered.contains("lang: pt"));
        assert!(rendered.contains("Não traduza"));
    }
}

use std::collections::HashMap;

use crate::compact::{render_ai_cards, AiCard};
use crate::error::MnemeError;
use crate::index::Index;
use crate::vault::Vault;

pub struct RecallOpts {
    pub query: String,
    pub tokens: usize,
    pub format: String,
}

pub fn recall(vault: &Vault, index: &Index, opts: &RecallOpts) -> Result<String, MnemeError> {
    let cards = index.cards()?;
    if cards.is_empty() {
        return Ok("# mneme recall\n(no notes — run `mneme reindex`)\n".into());
    }
    let mut fts: HashMap<String, f64> = HashMap::new();
    for (id, bm25) in index.fts_search(&opts.query)? {
        let score = 1.0 / (1.0 + bm25.abs());
        fts.insert(id, score);
    }
    let q = opts.query.to_lowercase();
    let neighbors = index.neighbors()?;
    let mut seed: HashMap<String, f64> = HashMap::new();
    for c in &cards {
        let mut s = 0.0;
        if let Some(f) = fts.get(&c.id) {
            s += 0.55 * f;
        }
        let hay = format!("{} {} {}", c.id, c.title, c.summary).to_lowercase();
        if !q.is_empty() && hay.contains(&q) {
            s += 0.25;
        }
        s += 0.2 * ((c.activation + 8.0) / 10.0).clamp(0.0, 1.0);
        if s > 0.0 {
            seed.insert(c.id.clone(), s);
        }
    }
    if seed.is_empty() {
        for c in cards.iter().take(opts.tokens.max(1)) {
            seed.insert(c.id.clone(), 0.05);
        }
    }
    let decay = vault.config.spread_decay;
    let mut spread = seed.clone();
    for (id, score) in &seed {
        if let Some(ns) = neighbors.get(id) {
            for n in ns {
                *spread.entry(n.clone()).or_insert(0.0) += score * decay;
            }
        }
    }
    let mut ranked = cards;
    ranked.sort_by(|a, b| {
        let sa = spread.get(&a.id).copied().unwrap_or(0.0);
        let sb = spread.get(&b.id).copied().unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.retain(|c| c.status != "obsolete");
    let wm = vault.config.working_memory.max(1);
    let mut picked = Vec::new();
    let mut used = 40usize;
    for c in ranked {
        if picked.len() >= wm {
            break;
        }
        let cost = c.token_est.min(180).max(20);
        if used + cost > opts.tokens && !picked.is_empty() {
            break;
        }
        used += cost;
        let links = index.links_from(&c.id).unwrap_or_default();
        let _ = index.record_access(&c.id, "recall");
        picked.push(AiCard {
            title: c.title,
            path: c.path,
            summary: c.summary,
            activation: c.activation,
            links,
            lang: c.lang,
        });
    }
    let _ = index.refresh_activation(&vault.config);
    match opts.format.as_str() {
        "json" => Ok(serde_json::to_string_pretty(&json_cards(&picked))?),
        _ => Ok(render_ai_cards(&opts.query, opts.tokens, used, &picked)),
    }
}

fn json_cards(cards: &[AiCard]) -> serde_json::Value {
    serde_json::json!(cards
        .iter()
        .map(|c| serde_json::json!({
            "title": c.title,
            "path": c.path,
            "summary": c.summary,
            "activation": c.activation,
            "links": c.links,
            "lang": c.lang,
        }))
        .collect::<Vec<_>>())
}

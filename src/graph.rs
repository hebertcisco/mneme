use std::collections::{HashMap, HashSet};
use std::fs;

use serde::Serialize;

use crate::atomic::atomic_write;
use crate::error::MnemeError;
use crate::note::extract_wikilinks;
use crate::vault::Vault;

#[derive(Serialize)]
struct GraphFile {
    generated: String,
    vault: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

#[derive(Serialize)]
struct GraphNode {
    id: String,
    label: String,
    kind: String,
    path: String,
    weight: i32,
}

#[derive(Serialize)]
struct GraphEdge {
    from: String,
    to: String,
}

pub fn export(vault: &Vault) -> Result<(usize, usize), MnemeError> {
    let notes = vault.iter_notes()?;
    let mut nodes: HashMap<String, GraphNode> = HashMap::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen = HashSet::new();
    for note in &notes {
        nodes.insert(
            note.id.clone(),
            GraphNode {
                id: note.id.clone(),
                label: note.id.clone(),
                kind: note.kind().to_string(),
                path: note.rel_path.clone(),
                weight: 0,
            },
        );
        for dst in extract_wikilinks(&note.raw) {
            nodes.entry(dst.clone()).or_insert(GraphNode {
                id: dst.clone(),
                label: dst.clone(),
                kind: "missing".into(),
                path: String::new(),
                weight: 0,
            });
            let key = format!("{}->{}", note.id, dst);
            if seen.insert(key) {
                if let Some(n) = nodes.get_mut(&note.id) {
                    n.weight += 1;
                }
                if let Some(n) = nodes.get_mut(&dst) {
                    n.weight += 1;
                }
                edges.push(GraphEdge {
                    from: note.id.clone(),
                    to: dst,
                });
            }
        }
    }
    let mut node_list: Vec<GraphNode> = nodes.into_values().collect();
    node_list.sort_by(|a, b| a.id.cmp(&b.id));
    let generated = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let payload = GraphFile {
        generated: generated.clone(),
        vault: vault.root.display().to_string(),
        nodes: node_list,
        edges,
    };
    let json = serde_json::to_string_pretty(&payload)?;
    let graph_dir = vault.root.join("graph");
    fs::create_dir_all(&graph_dir)?;
    atomic_write(&graph_dir.join("data.json"), json.as_bytes())?;
    let js = format!("window.GRAPH_DATA = {json};\n");
    atomic_write(&graph_dir.join("data.js"), js.as_bytes())?;
    patch_mermaid(vault, &payload)?;
    Ok((payload.nodes.len(), payload.edges.len()))
}

fn patch_mermaid(vault: &Vault, g: &GraphFile) -> Result<(), MnemeError> {
    let path = if vault.root.join("GRAPH.md").exists() {
        vault.root.join("GRAPH.md")
    } else {
        vault.root.join("GRAFO.md")
    };
    if !path.exists() {
        return Ok(());
    }
    let mut top: Vec<&GraphNode> = g.nodes.iter().collect();
    top.sort_by(|a, b| b.weight.cmp(&a.weight));
    top.truncate(28);
    let ids: HashSet<&str> = top.iter().map(|n| n.id.as_str()).collect();
    let mut lines = vec!["```mermaid".into(), "flowchart LR".into()];
    for n in &top {
        let safe = mermaid_id(&n.id);
        lines.push(format!("  {safe}[\"{}\"]", n.id.replace('"', "'")));
    }
    for e in &g.edges {
        if ids.contains(e.from.as_str()) && ids.contains(e.to.as_str()) {
            lines.push(format!("  {} --> {}", mermaid_id(&e.from), mermaid_id(&e.to)));
        }
    }
    lines.push("```".into());
    let mermaid = lines.join("\n");
    let block = format!("<!-- GRAFO:START -->\n{mermaid}\n<!-- GRAFO:END -->");
    let raw = fs::read_to_string(&path)?;
    let next = if raw.contains("<!-- GRAFO:START -->") {
        replace_block(&raw, &block)
    } else {
        format!("{}\n\n{block}\n", raw.trim_end())
    };
    atomic_write(&path, next.as_bytes())
}

fn replace_block(raw: &str, block: &str) -> String {
    let start = raw.find("<!-- GRAFO:START -->");
    let end = raw.find("<!-- GRAFO:END -->");
    match (start, end) {
        (Some(s), Some(e)) if e > s => {
            let mut out = String::new();
            out.push_str(&raw[..s]);
            out.push_str(block);
            out.push_str(&raw[e + "<!-- GRAFO:END -->".len()..]);
            out
        }
        _ => raw.to_string(),
    }
}

fn mermaid_id(id: &str) -> String {
    let mut s: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        s = format!("N_{s}");
    }
    s
}

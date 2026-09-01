use crate::error::MnemeError;
use crate::index::Index;
use crate::vault::Vault;

pub fn run(vault: &Vault, index: &Index) -> Result<String, MnemeError> {
    index.refresh_activation(&vault.config)?;
    let (nodes, edges) = crate::graph::export(vault)?;
    index.vacuum()?;
    Ok(format!(
        "consolidated: graph {nodes} nodes / {edges} edges, activation refreshed, sqlite vacuumed"
    ))
}

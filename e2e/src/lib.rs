#![cfg(test)]
#![cfg(not(target_arch = "wasm32"))]
use serde_json::json;
use workspaces::prelude::*;
use workspaces::{Contract, Network, Worker};

// Priority ordered mods
mod utils;

// Core runtime contracts
const MANAGER_WASM: &str = "../res/manager.wasm";

async fn manager_init(worker: Worker<impl Network>, contract: &Contract) -> anyhow::Result<()> {
    worker
        .call(
            contract,
            "new".to_string(),
            json!({}).to_string().into_bytes(),
            None,
        )
        .await?;

    let version: String = worker
        .view(contract.id().clone(), "version".to_string(), Vec::new())
        .await?
        .try_serde_deser()?;
    assert!(version.len() > 3, "No version found");

    Ok(())
}

#[tokio::test]
async fn main() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let agent = worker.dev_create().await?;
    println!("AGENT: {}", agent.id());

    let manager = utils::dev_deploy(worker.clone(), MANAGER_WASM)
        .await
        .expect("Manager deploy failed");

    println!("Manager ID: {}", manager.id());

    // initialize each contract with basics:
    manager_init(worker.clone(), &manager).await?;

    Ok(())
}

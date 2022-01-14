#![cfg(test)]
#![cfg(not(target_arch = "wasm32"))]
use serde_json::json;
use workspaces::{AccountId, Contract, Network, Worker};

// Bootstrap the core contract: Manager
pub async fn init_manager(
    worker: &Worker<impl Network>,
    contract: &Contract,
) -> anyhow::Result<()> {
    contract.call(&worker, "new").transact().await?;

    let result: String = contract
        .view(&worker, "version", Vec::new())
        .await?
        .json()?;
    println!("Manager Version: {}", result);
    assert_eq!(result.len(), 5);

    Ok(())
}

// Bootstrap the core contract: Rewards
pub async fn init_rewards(
    worker: &Worker<impl Network>,
    contract: &Contract,
    cron: &AccountId,
    dao: &AccountId,
) -> anyhow::Result<()> {
    contract
        .call(&worker, "new")
        .args_json(json!({
            "cron_account_id": cron,
            "dao_account_id": dao
        }))?
        .transact()
        .await?;

    let result: String = contract
        .view(&worker, "version", Vec::new())
        .await?
        .json()?;
    println!("Rewards Version: {}", result);
    assert_eq!(result.len(), 5);

    Ok(())
}

// Bootstrap the example contract: CRUD Cross Contract
pub async fn init_crudcross(
    worker: &Worker<impl Network>,
    contract: &Contract,
    cron: &AccountId,
) -> anyhow::Result<()> {
    contract
        .call(&worker, "new")
        .args_json(json!({ "cron": cron }))?
        .transact()
        .await?;

    // TODO:
    // let result: String = contract
    //     .view(&worker, "stats", Vec::new())
    //     .await?
    //     .json()?;
    // assert_eq!(resul);

    Ok(())
}

#![cfg(test)]
#![cfg(not(target_arch = "wasm32"))]
// use workspaces::prelude::*;
use workspaces::{Contract, Network, Worker};

// # Deploy all the contracts to their rightful places
// near deploy --wasmFile ./res/manager.wasm --accountId $CRON_ACCOUNT_ID --initFunction new --initArgs '{}'
// near deploy --wasmFile ./res/rewards.wasm --accountId $REWARDS_ACCOUNT_ID --initFunction new --initArgs '{"cron_account_id": "'$CRON_ACCOUNT_ID'", "dao_account_id": "'$DAO_ACCOUNT_ID'"}'
// near deploy --wasmFile ./res/rust_counter_tutorial.wasm --accountId $COUNTER_ACCOUNT_ID
// near deploy --wasmFile ./res/cross_contract.wasm --accountId $CRUD_ACCOUNT_ID --initFunction new --initArgs '{"cron": "'$CRON_ACCOUNT_ID'"}'


// Bootstrap the core contract: Manager
// #[tokio::test]
pub async fn init_manager(worker: &Worker<impl Network>, contract: &Contract) -> anyhow::Result<()> {
    let result = contract.call(&worker, "new")
        .args_json(serde_json::json!({}))?
        .transact()
        .await?;

    // TODO:
    // assert_eq!(result.status, SuccessValue, "Manager init failed");

    Ok(())
}

// Bootstrap the core contract: Manager
pub async fn init_rewards(worker: &Worker<impl Network>, contract: &Contract, cron: String, dao: String) -> anyhow::Result<()> {
    let result = contract.call(&worker, "new")
        .args_json(serde_json::json!({
            "cron_account_id": cron,
            "dao_account_id": dao
        }))?
        .transact()
        .await?;

    // TODO:
    // assert_eq!(result.status, SuccessValue, "Manager init failed");

    Ok(())
}

// Bootstrap the core contract: Manager
pub async fn init_crudcross(worker: &Worker<impl Network>, contract: &Contract, cron: String) -> anyhow::Result<()> {
    let result = contract.call(&worker, "new")
        .args_json(serde_json::json!({
            "cron": cron
        }))?
        .transact()
        .await?;

    // TODO:
    // assert_eq!(result.status, SuccessValue, "Manager init failed");

    Ok(())
}

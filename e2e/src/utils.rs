#![cfg(test)]
#![cfg(not(target_arch = "wasm32"))]
// use workspaces::prelude::*;
use near_units::parse_near;
use workspaces::{Account, Contract, Network, Worker};

// // helper that deploys a specific contract to mainnet
// pub async fn deploy(worker: Worker<impl Network>, contract_file: String) -> anyhow::Result<Contract> {
//     let file = std::fs::read(contract_file)?;
//     worker.deploy(file).await
// }

// // helper that deploys a specific contract
// // NOTE: `dev_deploy` is only available on `DevNetwork`s such sandbox and testnet.
// pub async fn dev_deploy(
//     worker: Worker<impl DevNetwork>,
//     contract_file: &str,
// ) -> anyhow::Result<Contract> {
//     let file = std::fs::read(contract_file)?;
//     worker.dev_deploy(file).await
// }

/// Creates a sub account & deploys the contract file to it
pub async fn create_subaccount(
    worker: &Worker<impl Network>,
    account: &Account,
    account_id: &str,
) -> anyhow::Result<Account> {
    // Create sub account
    let subaccount = account
        .create_subaccount(&worker, account_id)
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;
    assert_eq!(
        subaccount.id().to_string(),
        format!("{}.{}", account_id, account.id())
    );

    Ok(subaccount)
}

/// Creates a sub account & deploys the contract file to it
pub async fn create_subaccount_and_deploy_file(
    worker: &Worker<impl Network>,
    account: &Account,
    account_id: &str,
    file: &str,
) -> anyhow::Result<Contract> {
    let contract_file = std::fs::read(file)?;

    // Create sub account
    let subaccount = create_subaccount(&worker, &account, &account_id).await?;

    // Deploy
    let contract = subaccount.deploy(&worker, contract_file).await?.unwrap();

    Ok(contract)
}

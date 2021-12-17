#![cfg(test)]
#![cfg(not(target_arch = "wasm32"))]
use workspaces::prelude::*;

mod utils;
mod bootstrap;

// Core runtime contracts
const MANAGER_WASM: &str = "../res/manager.wasm";
const REWARDS_WASM: &str = "../res/rewards.wasm";
const CHARITY_WASM: &str = "../res/charity.wasm";
const COUNTER_WASM: &str = "../res/rust_counter_tutorial.wasm";
const CRUD_CROSS_WASM: &str = "../res/cross_contract.wasm";
// const SPUTNIKV2_WASM: &str = "../res/sputnikdao2.wasm";

// Doing all the setup before jumping into tests
#[tokio::test]
async fn main() -> anyhow::Result<()> {
    // TODO: Setup config for testnet/sandbox switching
    let worker = workspaces::sandbox();

    let croncat = worker.dev_create_account().await?;
    println!("CRONCAT: {}", croncat.id());

    let manager_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "manager_v1", MANAGER_WASM).await?;
    let rewards_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "rewards", REWARDS_WASM).await?;
    let charity_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "charity", CHARITY_WASM).await?;
    let counter_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "counter", COUNTER_WASM).await?;
    let crudcross_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "crudcross", CRUD_CROSS_WASM).await?;

    let users = worker.dev_create_account().await?;
    // let dao = worker.dev_create_account().await?;
    println!("users: {}", users.id());

    // NOTE: Adding many more agents in the future for diff scenarios, so naming convention has numbers
    let user_1 = utils::create_subaccount(&worker, &users, "user_1").await?;
    let agent_1 = utils::create_subaccount(&worker, &users, "agent_1").await?;
    println!("user_1: {}", user_1.id());
    println!("agent_1: {}", agent_1.id());

    println!("manager_contract id: {:?}", &manager_contract.id());
    println!("rewards_contract id: {:?}", &rewards_contract.id());
    println!("charity_contract id: {:?}", &charity_contract.id());
    println!("counter_contract id: {:?}", &counter_contract.id());
    println!("crudcross_contract id: {:?}", &crudcross_contract.id());

    // initialize each contract with basics:
    bootstrap::init_manager(&worker, &manager_contract).await?;
    // bootstrap::init_rewards(&worker, &manager_contract, manager_contract.id().to_string(), dao.id().to_string()).await?;
    // bootstrap::init_crudcross(&worker, &manager_contract, manager_contract.id().to_string()).await?;
    println!("crudcross_contract id: {:?}", &crudcross_contract.id());

    Ok(())
}

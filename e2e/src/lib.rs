#![cfg(test)]
#![cfg(not(target_arch = "wasm32"))]
use workspaces::prelude::*;

mod agents_basic;
mod bootstrap;
mod tasks;
mod utils;

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
    println!("CRONCAT: {}", &croncat.id());

    let manager_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "manager_v1", MANAGER_WASM)
            .await?;
    println!("manager_contract id: {}", &manager_contract.id());
    let rewards_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "rewards", REWARDS_WASM)
            .await?;
    println!("rewards_contract id: {}", &rewards_contract.id());
    let charity_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "charity", CHARITY_WASM)
            .await?;
    println!("charity_contract id: {}", &charity_contract.id());
    let counter_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "counter", COUNTER_WASM)
            .await?;
    println!("counter_contract id: {}", &counter_contract.id());
    let crudcross_contract =
        utils::create_subaccount_and_deploy_file(&worker, &croncat, "crudcross", CRUD_CROSS_WASM)
            .await?;
    println!("crudcross_contract id: {}", &crudcross_contract.id());

    let users = worker.dev_create_account().await?;
    let dao = worker.dev_create_account().await?;

    // NOTE: Adding many more agents in the future for diff scenarios, so naming convention has numbers
    let user_1 = utils::create_subaccount(&worker, &users, "user_1").await?;
    let agent_1 = utils::create_subaccount(&worker, &users, "agent_1").await?;
    println!("user_1: {}", user_1.id());
    println!("agent_1: {}", agent_1.id());

    // initialize each contract with basics:
    bootstrap::init_manager(&worker, &manager_contract).await?;
    bootstrap::init_rewards(
        &worker,
        &rewards_contract,
        &manager_contract.id(),
        &dao.id(),
    )
    .await?;
    bootstrap::init_crudcross(&worker, &crudcross_contract, &manager_contract.id()).await?;

    // Tasks
    tasks::lifecycle(&worker, &manager_contract, &user_1).await?;

    // Agent Flows
    agents_basic::lifecycle(&worker, &manager_contract, &agent_1).await?;

    Ok(())
}

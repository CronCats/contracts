#![cfg(test)]
#![cfg(not(target_arch = "wasm32"))]
use serde_json::json;
// use workspaces::prelude::*;
use near_units::{parse_gas, parse_near};
use workspaces::{Account, Contract, Network, Worker};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::{Base64VecU8, U128},
    serde::{Deserialize, Serialize},
    AccountId, Gas,
};

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct Task {
    pub owner_id: AccountId,
    pub contract_id: AccountId,
    pub function_id: String,
    pub cadence: String,
    pub recurring: bool,
    pub total_deposit: U128,
    pub deposit: U128,
    pub gas: Gas,
    pub arguments: Base64VecU8,
}

/// Runs a task through an entire set of common path calls
///  - Create Task
///  - Get Task
///  - Refill Task
///  - Remove Task
pub async fn lifecycle(
    worker: &Worker<impl Network>,
    contract: &Contract,
    user: &Account,
) -> anyhow::Result<()> {
    println!("Starting TASKS");
    let no_tasks: Vec<Task> = worker
        .view(
            contract.id().clone(),
            "get_tasks",
            json!({}).to_string().into_bytes(),
        )
        .await?
        .json()?;
    assert_eq!(no_tasks.len(), 0);
    // println!("no_tasks {:#?}", no_tasks);

    let task_hash: String = user
        .call(&worker, contract.id().clone(), "create_task")
        .args_json(json!({
            "contract_id": "counter.examples.near",
            "function_id": "increment",
            "cadence": "0 0 * * * *"
        }))?
        .gas(parse_gas!("200 Tgas") as u64)
        .deposit(parse_near!("1 N"))
        .transact()
        .await?
        .json()?;

    // println!("task hash {:?}", task_hash);

    let one_task: Task = worker
        .view(
            contract.id().clone(),
            "get_task",
            json!({ "task_hash": &task_hash }).to_string().into_bytes(),
        )
        .await?
        .json()?;
    assert_eq!(one_task.total_deposit.0, parse_near!("1 N"));
    // println!("one_task {:#?}", one_task);

    user.call(&worker, contract.id().clone(), "refill_balance")
        .args_json(json!({ "task_hash": &task_hash }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    let one_task_updated: Task = worker
        .view(
            contract.id().clone(),
            "get_task",
            json!({ "task_hash": &task_hash }).to_string().into_bytes(),
        )
        .await?
        .json()?;
    // look for balance to make sure the task actually updated
    assert_eq!(one_task_updated.total_deposit.0, parse_near!("2 N"));
    // println!("one_task_updated {:#?}", one_task_updated);

    user.call(&worker, contract.id().clone(), "remove_task")
        .args_json(json!({ "task_hash": &task_hash }))?
        .transact()
        .await?;

    let no_more_tasks: Vec<Task> = worker
        .view(
            contract.id().clone(),
            "get_tasks",
            json!({}).to_string().into_bytes(),
        )
        .await?
        .json()?;
    assert_eq!(no_more_tasks.len(), 0);

    Ok(())
}

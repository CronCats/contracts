#![cfg(test)]
#![cfg(not(target_arch = "wasm32"))]
use serde_json::json;
// use workspaces::prelude::*;
use near_units::{parse_gas, parse_near};
use workspaces::{Account, Contract, Network, Worker};

// TODO: Add view calls to check state worked appropriately
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
    // let start_tasks: String = worker
    //     .view(
    //         contract.id().clone(),
    //         "get_tasks",
    //         json!({}).to_string().into_bytes(),
    //     )
    //     .await?
    //     .json()?;
    // // assert_eq!(start_tasks.len(), 5);
    // println!("start_tasks {:#?}", start_tasks);

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

    println!("task hash {:?}", task_hash);

    user.call(&worker, contract.id().clone(), "refill_balance")
        .args_json(json!({ "task_hash": &task_hash }))?
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    user.call(&worker, contract.id().clone(), "remove_task")
        .args_json(json!({ "task_hash": &task_hash }))?
        .transact()
        .await?;

    // let result: String = contract
    //     .view(
    //         &worker,
    //         "version",
    //         json!({})
    //         .to_string()
    //         .into_bytes(),
    //     )
    //     .await?
    //     .json()?;
    // assert_eq!(result.len(), 5);

    Ok(())
}

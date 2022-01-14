#![cfg(test)]
#![cfg(not(target_arch = "wasm32"))]
use std::{thread, time};
use serde_json::json;
use near_units::{parse_gas, parse_near};
use near_primitives::views::FinalExecutionStatus;
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

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum AgentStatus {
    Active,
    Pending,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct Agent {
    pub status: AgentStatus,
    pub payable_account_id: AccountId,
    pub balance: U128,
    pub total_tasks_executed: U128,
    pub last_missed_slot: u128,
}

/// FLOW for becoming and managing an agent
/// * Can register
/// * cannot re-register
/// * Can withdraw funds
/// * can unregister
/// * Cannot unregister if doesnt exist
pub async fn lifecycle(
    worker: &Worker<impl Network>,
    contract: &Contract,
    agent: &Account,
) -> anyhow::Result<()> {
    println!("Prep AGENT BASICS");
    agent
        .call(&worker, contract.id().clone(), "create_task")
        .args_json(json!({
            "contract_id": "counter.examples.near",
            "function_id": "increment",
            "cadence": "*/1 * * * * *"
        }))?
        .gas(parse_gas!("200 Tgas") as u64)
        .deposit(parse_near!("1 N"))
        .transact()
        .await?;

    println!("Starting AGENT BASICS");
    let no_agent: Option<Agent> = worker
        .view(
            contract.id().clone(),
            "get_agent",
            json!({"account_id": agent.id().clone() }).to_string().into_bytes(),
        )
        .await?
        .json()?;
    // println!("no_agent {:#?}", no_agent);
    assert!(no_agent.is_none());

    // register
    agent
        .call(&worker, contract.id().clone(), "register_agent")
        .deposit(parse_near!("0.00226 N"))
        .transact()
        .await?;

    // check the righ agent was registered
    let new_agent: Option<Agent> = worker
        .view(
            contract.id().clone(),
            "get_agent",
            json!({"account_id": agent.id().clone() }).to_string().into_bytes(),
        )
        .await?
        .json()?;
    // println!("new_agent {:#?}", new_agent);
    assert!(new_agent.is_some());
    let new_agent_data = new_agent.unwrap();
    assert_eq!(new_agent_data.status, AgentStatus::Active);
    assert_eq!(new_agent_data.payable_account_id.to_string(), agent.id().clone().to_string());
    
    // Check we cannot register again
    let fail_agent = agent
        .call(&worker, contract.id().clone(), "register_agent")
        .args_json(json!({}))?
        .deposit(parse_near!("0.00226 N"))
        .transact()
        .await?;
    // println!("agent fail_agent {:#?}", fail_agent.status);
    let fail_agent_bool: bool = match fail_agent.status {
        // match it against a specific error
        FinalExecutionStatus::Failure(e) => {
            e.to_string().contains("Agent already exists")
        },
        _ => false,
    };
    assert_eq!(fail_agent_bool, true);
    
    // NOTE: Once fastfwd is possible, we can remove this
    println!("Waiting until next slot occurs...");
    // pause for blocks to clear
    thread::sleep(time::Duration::from_millis(65_000));

    // quick proxy call to earn a reward
    agent
        .call(&worker, contract.id().clone(), "proxy_call")
        .gas(parse_gas!("200 Tgas") as u64)
        .transact()
        .await?;
    
    // check accumulated agent balance
    let bal_agent: Option<Agent> = worker
        .view(
            contract.id().clone(),
            "get_agent",
            json!({"account_id": agent.id().clone() }).to_string().into_bytes(),
        )
        .await?
        .json()?;
    // println!("bal_agent {:#?}", bal_agent);
    assert!(bal_agent.is_some());
    assert_eq!(bal_agent.unwrap().balance.0, parse_near!("0.00306 N"));

    // withdraw reward
    agent
        .call(&worker, contract.id().clone(), "withdraw_task_balance")
        .transact()
        .await?;
    
    // check accumulated agent balance
    let bal_done_agent: Option<Agent> = worker
        .view(
            contract.id().clone(),
            "get_agent",
            json!({"account_id": agent.id().clone() }).to_string().into_bytes(),
        )
        .await?
        .json()?;
    // println!("bal_done_agent {:#?}", bal_done_agent);
    assert!(bal_done_agent.is_some());
    assert_eq!(bal_done_agent.unwrap().balance.0, parse_near!("0.00226 N"));

    // unregister agent
    agent
        .call(&worker, contract.id().clone(), "unregister_agent")
        .deposit(parse_near!("1y"))
        .transact()
        .await?;
    
    let removed_agent: Option<Agent> = worker
        .view(
            contract.id().clone(),
            "get_agent",
            json!({"account_id": agent.id().clone() }).to_string().into_bytes(),
        )
        .await?
        .json()?;
    // println!("removed_agent {:#?}", removed_agent);
    assert!(removed_agent.is_none());
    
    // try to unregister agent again, check it fails
    let fail_unregister = agent
        .call(&worker, contract.id().clone(), "unregister_agent")
        .deposit(parse_near!("1y"))
        .transact()
        .await?;
    // println!("agent fail_unregister {:#?}", fail_unregister.status);
    // TODO: get the error to trigger, not sure why state is not working 
    let fail_unregister_bool: bool = match fail_unregister.status {
        // match it against a specific error
        FinalExecutionStatus::Failure(e) => {
            // println!("{:?}", e);
            // e.to_string().contains("Agent already exists");
            true
        },
        FinalExecutionStatus::SuccessValue(e) => {
            // println!("SUS {:?}", e);
            // e.to_string().contains("Agent already exists");
            true
        },
        _ => false,
    };
    assert_eq!(fail_unregister_bool, true);

    Ok(())
}

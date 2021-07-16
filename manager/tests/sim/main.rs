mod test_utils;

use crate::test_utils::{
    bootstrap_time_simulation, counter_create_task, find_log_from_outcomes, helper_create_task,
    sim_helper_create_agent_user, sim_helper_init, sim_helper_init_counter,
};
use manager::{Agent, Task};
use near_sdk::{json_types::{Base64VecU8, U128}};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json;
use near_sdk::serde_json::{json, Value};
use near_sdk_sim::hash::CryptoHash;
use near_sdk_sim::transaction::{ExecutionStatus, SignedTransaction};
use near_sdk_sim::DEFAULT_GAS;

// Load in contract bytes at runtime
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    pub CRON_MANAGER_WASM_BYTES => "../target/wasm32-unknown-unknown/release/manager.wasm",
    pub COUNTER_WASM_BYTES => "../target/wasm32-unknown-unknown/release/rust_counter_tutorial.wasm",
}

const MANAGER_ID: &str = "manager.sim";
const COUNTER_ID: &str = "counter.sim";
const AGENT_ID: &str = "agent.sim";
const USER_ID: &str = "user.sim";
const TASK_BASE64: &str = "chUCZxP6uO5xZIjwI9XagXVUCV7nmE09HVRUap8qauo=";
const AGENT_REGISTRATION_COST: u128 = 2_090_000_000_000_000_000_000;
const AGENT_FEE: u128 = 60_000_000_000_000_000_000_000u128;

type TaskBase64Hash = String;

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct GetTasksReturn {
    hashes: Vec<Base64VecU8>,
    slot: U128,
}

#[test]
fn simulate_task_creation() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);
}

/// Creates 11 tasks that will occupy different slots, have agent execute them.
#[test]
fn simulate_many_tasks() {
    let (agent_signer, root_account, agent, counter, cron) = bootstrap_time_simulation();

    // Increase agent fee a bit
    cron.call(
        cron.account_id(),
        "update_settings",
        &json!({ "agent_fee": U128::from(AGENT_FEE) })
            .to_string()
            .into_bytes(), // 0.06 Ⓝ
        DEFAULT_GAS,
        0, // attached deposit
    )
    .assert_success();

    // Create 11 tasks with different cadences
    counter_create_task(&counter, cron.account_id(), "0 3 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "5 5 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "6 7 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "19 11 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "6 * 3 * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "0 13 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "6 19 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "6 31 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "0 47 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "0 7 5 * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "0 43 * * * * *").assert_success();

    // Slots 120, 240, 360, 600, 720, 1080, 1800, 2520, 2760, 10740, 18360

    // register agent
    agent
        .call(
            "cron.root".to_string(),
            "register_agent",
            &json!({}).to_string().into_bytes(),
            DEFAULT_GAS,
            AGENT_REGISTRATION_COST,
        )
        .assert_success();

    // Here's where things get interesting. We must borrow mutable runtime
    // in order to move blocks forward. But once we do, future calls will
    // look different.
    let mut root_runtime = root_account.borrow_runtime_mut();
    assert!(
        root_runtime.produce_blocks(120).is_ok(),
        "Couldn't produce blocks"
    );

    // Should find a task
    let mut get_tasks_view_res =
        root_runtime.view_method_call("cron.root", "get_tasks", "{}".as_bytes());
    let mut success_val = r#"
        [["/YD9yxy6pZjlvra3qkvybKdodL3alsfvR6S62/FiYow="],"120"]
    "#;
    let mut success_vec: Vec<u8> = success_val.trim().into(); // trim because of multiline assignment above
    assert_eq!(
        get_tasks_view_res.unwrap(),
        success_vec,
        "Should find one particular task hash at slot 120"
    );

    // Check that the counter really did update
    let get_counter_view_res =
        root_runtime.view_method_call("counter.root", "get_num", "{}".as_bytes()).unwrap();
    assert_eq!(get_counter_view_res[0], 48, "Counter number before proxy");

    // Agent calls proxy_call using new transaction syntax with borrowed,
    // mutable runtime object.
    let mut res = root_runtime.resolve_tx(SignedTransaction::call(
        3,
        "agent.root".to_string(),
        "cron.root".to_string(),
        &agent_signer.clone(),
        0,
        "proxy_call".into(),
        "{}".as_bytes().to_vec(),
        DEFAULT_GAS,
        CryptoHash::default(),
    ));
    let (_, res_outcome) = res.unwrap();
    assert_eq!(res_outcome.status, ExecutionStatus::SuccessValue(vec![]));

    // Check that the counter really did update
    let get_counter_view_res =
        root_runtime.view_method_call("counter.root", "get_num", "{}".as_bytes()).unwrap();
    assert_eq!(get_counter_view_res[0], 49, "Counter updated from proxy call");

    // Ensure it doesn't find tasks now, except for the same one that's now completed
    get_tasks_view_res = root_runtime.view_method_call("cron.root", "get_tasks", "{}".as_bytes());
    success_val = r#"
        [[],"120"]
    "#;
    success_vec = success_val.trim().into();
    assert_eq!(
        get_tasks_view_res.unwrap(),
        success_vec,
        "Should find no task hashes at slot 120 anymore"
    );

    let mut tasks_info: GetTasksReturn = get_tasks_view_res.unwrap_json();
    assert_eq!(tasks_info.hashes.len(), 0, "Expected no tasks as before");

    success_vec = success_val.trim().into();
    assert_eq!(
        get_tasks_view_res.unwrap(),
        success_vec,
        "There should not be any tasks at current slot of 120"
    );

    // Proxy call should panic when no tasks to execute
    res = root_runtime.resolve_tx(SignedTransaction::call(
        4,
        "agent.root".to_string(),
        "cron.root".to_string(),
        &agent_signer.clone(),
        0,
        "proxy_call".into(),
        "{}".as_bytes().to_vec(),
        DEFAULT_GAS,
        CryptoHash::default(),
    ));
    let (_, res_outcome) = res.unwrap();
    // Ensure that it panics with a message we expect.
    match res_outcome.status {
        ExecutionStatus::Failure(f) => {
            // Not great to use `contains` but will have to do for now.
            assert!(
                f.to_string().contains("No tasks found in slot"),
                "Should have error that no tasks are available"
            );
        }
        _ => panic!("Expected failure when proxy_call has no tasks to execute"),
    }

    // Go through the remainder of the slots, executing tasks
    let mut nonce = 4;
    for n in &[240, 360, 600, 720, 1080, 1800, 2520, 2760, 10740, 18360] {
        // produce blocks until next slot
        let cur_block_height = root_runtime.cur_block.block_height;
        assert!(
            root_runtime.produce_blocks(n - cur_block_height).is_ok(),
            "Couldn't produce blocks"
        );
        get_tasks_view_res =
            root_runtime.view_method_call("cron.root", "get_tasks", "{}".as_bytes());
        tasks_info = get_tasks_view_res.unwrap_json();
        assert_eq!(tasks_info.hashes.len(), 1, "Expecting 1 task for this slot");
        // Proxy call
        nonce += 1;
        res = root_runtime.resolve_tx(SignedTransaction::call(
            nonce,
            "agent.root".to_string(),
            "cron.root".to_string(),
            &agent_signer.clone(),
            0,
            "proxy_call".into(),
            "{}".as_bytes().to_vec(),
            DEFAULT_GAS,
            CryptoHash::default(),
        ));
        let (_, res_outcome) = res.unwrap();
        assert_eq!(
            res_outcome.status,
            ExecutionStatus::SuccessValue(vec![]),
            "Expected proxy_call to succeed when looping through."
        );
    }

    let mut agent_info_result = root_runtime.view_method_call(
        "cron.root",
        "get_agent",
        "{\"account\": \"agent.root\"}".as_bytes(),
    );
    let mut agent_info: Agent = agent_info_result.unwrap_json();
    // Confirm that the agent has executed 11 tasks
    assert_eq!(
        agent_info.total_tasks_executed.0, 11,
        "Expected agent to have completed 11 tasks."
    );

    // Agent withdraws balance, claiming rewards
    nonce += 1;
    root_runtime
        .resolve_tx(SignedTransaction::call(
            nonce,
            "agent.root".to_string(),
            "cron.root".to_string(),
            &agent_signer,
            0,
            "withdraw_task_balance".into(),
            "{}".as_bytes().to_vec(),
            DEFAULT_GAS,
            CryptoHash::default(),
        ))
        .expect("Error withdrawing task balance");

    nonce += 1;
    let res2 = root_runtime.resolve_tx(SignedTransaction::call(
        nonce,
        "agent.root".to_string(),
        "cron.root".to_string(),
        &agent_signer,
        0,
        "get_agent".into(),
        "{\"account\": \"agent.root\"}".as_bytes().to_vec(),
        DEFAULT_GAS,
        CryptoHash::default(),
    ));
    let (_, res_outcome2) = res2.unwrap();
    let new_agent_balance2 = match res_outcome2.status {
        ExecutionStatus::SuccessValue(res_agent) => {
            let res_agent_info = String::from_utf8_lossy(res_agent.as_ref());
            let agent: Agent = serde_json::from_str(res_agent_info.as_ref()).unwrap();
            agent.balance
        }
        _ => panic!("Did not successfully get agent info"),
    };
    // println!("new_agent_balance2 {}", new_agent_balance2.0);
    assert_eq!(new_agent_balance2.0, AGENT_REGISTRATION_COST);

    // let expected_log = format!("Withdrawal of {} has been sent.", AGENT_FEE * 11);
    // find_log_from_outcomes(&root_runtime, &expected_log.to_string());

    // Ensure that there's no balance for agent now
    agent_info_result = root_runtime.view_method_call(
        "cron.root",
        "get_agent",
        "{\"account\": \"agent.root\"}".as_bytes(),
    );
    agent_info = agent_info_result.unwrap_json();
    assert_eq!(
        agent_info.balance,
        U128::from(AGENT_REGISTRATION_COST),
        "Agent balance should be only state storage after withdrawal."
    );

    // Unregister agent and ensure it's removed
    nonce += 1;
    root_runtime
        .resolve_tx(SignedTransaction::call(
            nonce,
            "agent.root".to_string(),
            "cron.root".to_string(),
            &agent_signer,
            1,
            "unregister_agent".into(),
            "{}".as_bytes().to_vec(),
            DEFAULT_GAS,
            CryptoHash::default(),
        ))
        .expect("Issue with agent unregister transaction");

    // Check that the proper amount was refunded
    // + 1 because of the yoctoⓃ that was attached above
    let expected_log = format!(
        "Agent has been removed and refunded the storage cost of {}",
        AGENT_REGISTRATION_COST + 1
    );
    find_log_from_outcomes(&root_runtime, &expected_log.to_string());

    agent_info_result = root_runtime.view_method_call(
        "cron.root",
        "get_agent",
        "{\"account\": \"agent.root\"}".as_bytes(),
    );
    assert!(agent_info_result.is_ok(), "Expected get_agent to return Ok");
    let agent_info_val: Value = agent_info_result.unwrap_json_value();

    assert_eq!(
        agent_info_val,
        Value::Null,
        "Expected a null return for the agent meaning it no longer exists."
    );
}

#[test]
fn simulate_basic_task_checks() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);

    // Nonexistent task fails.
    let mut task_view_result = root.view(
        cron.account_id(),
        "get_task",
        &json!({
            "task_hash": "doesnotexist"
        })
        .to_string()
        .into_bytes(),
    );
    assert!(
        task_view_result.is_err(),
        "Expected nonexistent task to throw error."
    );
    let error = task_view_result.unwrap_err();
    let error_message = error.to_string();
    assert!(error_message.contains("No task found by hash"));

    // Get hash from task just added.
    task_view_result = root.view(
        cron.account_id(),
        "get_task",
        &json!({ "task_hash": TASK_BASE64 }).to_string().into_bytes(),
    );
    assert!(
        task_view_result.is_ok(),
        "Expected to find hash of task just added."
    );
    let returned_task: Task = task_view_result.unwrap_json();

    let expected_task = Task {
        owner_id: COUNTER_ID.to_string(),
        contract_id: COUNTER_ID.to_string(),
        function_id: "increment".to_string(),
        cadence: "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2".to_string(),
        recurring: true,
        total_deposit: U128::from(6030000000000000),
        deposit: U128::from(12000000000000),
        gas: 3000000000000,
        arguments: vec![],
    };
    assert_eq!(
        expected_task, returned_task,
        "Task returned was not expected."
    );

    // Attempt to remove task with non-owner account.
    let removal_result = root.call(
        cron.account_id(),
        "remove_task",
        &json!({ "task_hash": TASK_BASE64 }).to_string().into_bytes(),
        DEFAULT_GAS,
        0, // deposit
    );
    let status = removal_result.status();
    if let ExecutionStatus::Failure(err) = status {
        // At this time, this is the way to check for error messages.
        assert!(err
            .to_string()
            .contains("Only owner can remove their task."));
    } else {
        panic!("Non-owner account should not succeed in removing task.");
    }

    counter
        .call(
            cron.account_id(),
            "remove_task",
            &json!({ "task_hash": TASK_BASE64 }).to_string().into_bytes(),
            DEFAULT_GAS,
            0, // deposit
        )
        .assert_success();

    // Get hash from task just removed.
    task_view_result = root.view(
        cron.account_id(),
        "get_task",
        &json!({ "task_hash": TASK_BASE64 }).to_string().into_bytes(),
    );
    assert!(
        task_view_result.is_err(),
        "Expected error when trying to retrieve removed task."
    );
}

#[test]
fn simulate_basic_agent_registration_update() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);
    let (agent, _) = sim_helper_create_agent_user(&root);

    // Register an agent, where the beneficiary is user.sim
    agent
        .call(
            cron.account_id(),
            "register_agent",
            &json!({ "payable_account_id": USER_ID })
                .to_string()
                .into_bytes(),
            DEFAULT_GAS,
            AGENT_REGISTRATION_COST, // deposit
        )
        .assert_success();

    // Attempt to re-register
    let mut failed_result = agent.call(
        cron.account_id(),
        "register_agent",
        &json!({ "payable_account_id": USER_ID })
            .to_string()
            .into_bytes(),
        DEFAULT_GAS,
        AGENT_REGISTRATION_COST, // deposit
    );
    let mut status = failed_result.status();
    if let ExecutionStatus::Failure(err) = status {
        // At this time, this is the way to check for error messages.
        assert!(err.to_string().contains("Agent already exists"));
    } else {
        panic!("Should not be able to re-register an agent.");
    }

    // Update agent with an invalid name
    failed_result = agent.call(
        cron.account_id(),
        "update_agent",
        &json!({
            "payable_account_id": "inv*lid.n@me"
        })
        .to_string()
        .into_bytes(),
        DEFAULT_GAS,
        0, // deposit
    );

    status = failed_result.status();
    if let ExecutionStatus::Failure(err) = status {
        // At this time, this is the way to check for error messages.
        assert!(err.to_string().contains("The account ID is invalid"));
    } else {
        panic!("Should not be able to send invalid account ID.");
    }

    // Update agent with a valid account name
    agent.call(
        cron.account_id(),
        "update_agent",
        &json!({
            "payable_account_id": "newname.sim"
        })
        .to_string()
        .into_bytes(),
        DEFAULT_GAS,
        0, // deposit
    );

    let agent_result: Agent = root
        .view(
            cron.account_id(),
            "get_agent",
            &json!({
                "account": agent.account_id
            })
            .to_string()
            .into_bytes(),
        )
        .unwrap_json();

    assert_eq!(agent_result.payable_account_id, "newname.sim".to_string());
}

#[test]
fn simulate_agent_unregister_check() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);
    let unregister_result = cron.call(cron.account_id(), "unregister_agent", &[], DEFAULT_GAS, 1);
    unregister_result.assert_success();
    for log in unregister_result.logs() {
        assert_eq!(log, &"The agent manager.sim is not registered".to_string());
    }
}

#[test]
fn simulate_task_creation_agent_usage() {
    let (agent_signer, root_account, agent, counter, cron) = bootstrap_time_simulation();

    // Increase agent fee a bit
    cron.call(
        cron.account_id(),
        "update_settings",
        &json!({ "agent_fee": U128::from(AGENT_FEE) })
            .to_string()
            .into_bytes(), // 0.06 Ⓝ
        DEFAULT_GAS,
        0, // attached deposit
    )
    .assert_success();

    // create a task
    let execution_result = counter_create_task(&counter, "cron.root".to_string(), "0 30 * * * * *");
    execution_result.assert_success();

    // register agent
    agent
        .call(
            "cron.root".to_string(),
            "register_agent",
            &json!({}).to_string().into_bytes(),
            DEFAULT_GAS,
            AGENT_REGISTRATION_COST,
        )
        .assert_success();

    // Here's where things get interesting. We must borrow mutable runtime
    // in order to move blocks forward. But once we do, future calls will
    // look different.
    let mut root_runtime = root_account.borrow_runtime_mut();
    // Move forward proper amount until slot 1740
    let block_production_result = root_runtime.produce_blocks(1780);
    assert!(block_production_result.is_ok(), "Couldn't produce blocks");

    // Agent calls proxy_call using new transaction syntax with borrowed,
    // mutable runtime object.
    let mut res = root_runtime.resolve_tx(SignedTransaction::call(
        6, // I don't think this matters
        "agent.root".to_string(),
        "cron.root".to_string(),
        &agent_signer.clone(),
        0,
        "proxy_call".into(),
        "{}".as_bytes().to_vec(),
        DEFAULT_GAS,
        CryptoHash::default(),
    ));
    let (_, res_outcome) = res.unwrap();
    assert_eq!(res_outcome.status, ExecutionStatus::SuccessValue(vec![]));

    // Look at agent object and see how much balance there is
    res = root_runtime.resolve_tx(SignedTransaction::call(
        8,
        "agent.root".to_string(),
        "cron.root".to_string(),
        &agent_signer,
        0,
        "get_agent".into(),
        "{\"account\": \"agent.root\"}".as_bytes().to_vec(),
        DEFAULT_GAS,
        CryptoHash::default(),
    ));
    let (_, res_outcome) = res.unwrap();
    let new_agent_balance = match res_outcome.status {
        ExecutionStatus::SuccessValue(res_agent) => {
            let res_agent_info = String::from_utf8_lossy(res_agent.as_ref());
            let agent: Agent = serde_json::from_str(res_agent_info.as_ref()).unwrap();
            agent.balance
        }
        _ => panic!("Did not successfully get agent info"),
    };
    // The agent's balance should be the storage cost plus the reward
    // assert_eq!(new_agent_balance.0, AGENT_REGISTRATION_COST + AGENT_FEE);
    assert_eq!(new_agent_balance.0, 62217222281030900000000); // NOTE: the above needs to change to gas used * gas price in addition to registration and fee.

    // Agent withdraws balance, claiming rewards
    // Here we don't resolve the transaction, but instead just send it so we can view
    // the receipts generated
    root_runtime
        .resolve_tx(SignedTransaction::call(
            9,
            "agent.root".to_string(),
            "cron.root".to_string(),
            &agent_signer,
            0,
            "withdraw_task_balance".into(),
            "{}".as_bytes().to_vec(),
            DEFAULT_GAS,
            CryptoHash::default(),
        ))
        .expect("Error withdrawing task balance");

    let res2 = root_runtime.resolve_tx(SignedTransaction::call(
        10,
        "agent.root".to_string(),
        "cron.root".to_string(),
        &agent_signer,
        0,
        "get_agent".into(),
        "{\"account\": \"agent.root\"}".as_bytes().to_vec(),
        DEFAULT_GAS,
        CryptoHash::default(),
    ));
    let (_, res_outcome2) = res2.unwrap();
    let new_agent_balance2 = match res_outcome2.status {
        ExecutionStatus::SuccessValue(res_agent) => {
            let res_agent_info = String::from_utf8_lossy(res_agent.as_ref());
            let agent: Agent = serde_json::from_str(res_agent_info.as_ref()).unwrap();
            agent.balance
        }
        _ => panic!("Did not successfully get agent info"),
    };
    // println!("new_agent_balance2 {}", new_agent_balance2.0);
    assert_eq!(new_agent_balance2.0, AGENT_REGISTRATION_COST);

    // Look for this log
    // let expected_log = format!("Withdrawal of {} has been sent.", AGENT_FEE);
    // find_log_from_outcomes(&root_runtime, &expected_log.to_string());
}

mod test_utils;

use crate::test_utils::{
    bootstrap_time_simulation, counter_create_task, find_log_from_outcomes, helper_create_task,
    sim_helper_create_agent_user, sim_helper_init, sim_helper_init_counter,
    sim_helper_init_sputnikv2,
};
use manager::{Agent, TaskHumanFriendly};
use near_sdk::json_types::{Base64VecU8, U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json;
use near_sdk::serde_json::{json, Value};
use near_sdk_sim::hash::CryptoHash;
use near_sdk_sim::transaction::{ExecutionStatus, SignedTransaction};
use near_sdk_sim::types::AccountId;
use near_sdk_sim::{to_yocto, DEFAULT_GAS};

// Load in contract bytes at runtime
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    pub CRON_MANAGER_WASM_BYTES => "../target/wasm32-unknown-unknown/release/manager.wasm",
    pub COUNTER_WASM_BYTES => "../target/wasm32-unknown-unknown/release/rust_counter_tutorial.wasm",
    pub SPUTNIKV2_WASM_BYTES => "./tests/sputnik/sputnikdao2.wasm",
}

const MANAGER_ID: &str = "manager.sim";
const COUNTER_ID: &str = "counter.sim";
const SPUTNIKV2_ID: &str = "sputnikv2.sim";
const AGENT_ID: &str = "agent.sim";
const USER_ID: &str = "user.sim";
const NEW_NAME_ID: &str = "newname.sim";
const TASK_BASE64: &str = "BBcr1GdY4iSMebFavu7yz4daPDDrlmxTf5ftC0RB8mQ=";
const AGENT_REGISTRATION_COST: u128 = 2_260_000_000_000_000_000_000;
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

    // Slots ["240000000000","360000000000","480000000000","720000000000","840000000000","1200000000000","1920000000000","2640000000000","2880000000000","10860000000000","18480000000000"]

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
    while root_runtime.produce_blocks(1).is_ok() {
        if root_runtime.cur_block.block_timestamp >= 240000000000 {
            break;
        }
    }

    // Should find a task
    let mut get_tasks_view_res =
        root_runtime.view_method_call("cron.root", "get_slot_tasks", "{\"offset\": 1}".as_bytes());
    // println!("get_tasks_view_res {:?}", get_tasks_view_res);
    // let mut success_val = r#"
    //     [["xdnWQtc0KAq2i+/vyFQSHGvr5K0DPgyVUYfE8886qMs="],"240000000000"]
    // "#;
    let success_vecs: Vec<u8> = vec![
        91, 91, 34, 50, 50, 71, 50, 90, 108, 84, 111, 119, 47, 52, 86, 105, 70, 68, 119, 70, 98,
        72, 109, 97, 49, 51, 112, 87, 120, 118, 52, 111, 66, 122, 111, 114, 68, 111, 88, 112, 72,
        53, 79, 97, 120, 56, 61, 34, 93, 44, 34, 51, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
        34, 93,
    ];
    assert_eq!(
        get_tasks_view_res.unwrap(),
        success_vecs,
        "Should find one particular task hash at slot 240000000000"
    );

    // Check that the counter really did update
    let get_counter_view_res = root_runtime
        .view_method_call("counter.root", "get_num", "{}".as_bytes())
        .unwrap();
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
    let get_counter_view_res = root_runtime
        .view_method_call("counter.root", "get_num", "{}".as_bytes())
        .unwrap();
    assert_eq!(
        get_counter_view_res[0], 49,
        "Counter updated from proxy call"
    );

    // Ensure it doesn't find tasks now, except for the same one that's now completed
    get_tasks_view_res =
        root_runtime.view_method_call("cron.root", "get_slot_tasks", "{}".as_bytes());
    let success_val = r#"
        [[],"240000000000"]
    "#;
    let mut success_vec: Vec<u8> = success_val.trim().into(); // trim because of multiline assignment above
    assert_eq!(
        get_tasks_view_res.unwrap(),
        success_vec,
        "Should find no task hashes at slot 240000000000 anymore"
    );

    let mut tasks_info: GetTasksReturn = get_tasks_view_res.unwrap_json();
    assert_eq!(tasks_info.hashes.len(), 0, "Expected no tasks as before");

    success_vec = success_val.trim().into();
    assert_eq!(
        get_tasks_view_res.unwrap(),
        success_vec,
        "There should not be any tasks at current slot of 240000000000"
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
    println!("{:?}", res);
    // NOTE: Disabled because the logic needs to handle failures gracefully to reward agent
    // let (_, res_outcome) = res.unwrap();
    // Ensure that it panics with a message we expect.
    // match res_outcome.status {
    //     ExecutionStatus::Failure(f) => {
    //         // Not great to use `contains` but will have to do for now.
    //         assert!(
    //             f.to_string().contains("No tasks found in slot"),
    //             "Should have error that no tasks are available"
    //         );
    //     }
    //     _ => panic!("Expected failure when proxy_call has no tasks to execute"),
    // }

    // Go through the remainder of the slots, executing tasks
    let mut nonce = 4;
    for n in &[
        360000000000u64,
        480000000000u64,
        720000000000u64,
        840000000000u64,
        1200000000000u64,
        1920000000000u64,
        2640000000000u64,
        2880000000000u64,
        10860000000000u64,
        18480000000000u64,
    ] {
        // produce blocks until next slot
        while root_runtime.produce_blocks(1).is_ok() {
            if &root_runtime.cur_block.block_timestamp >= n {
                break;
            }
        }
        get_tasks_view_res =
            root_runtime.view_method_call("cron.root", "get_slot_tasks", "{}".as_bytes());
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
        "{\"account_id\": \"agent.root\"}".as_bytes(),
    );
    let mut agent_info: Agent = agent_info_result.unwrap_json();
    // Confirm that the agent has executed 12 tasks
    assert_eq!(
        agent_info.total_tasks_executed.0, 12,
        "Expected agent to have completed 12 tasks."
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
        "{\"account_id\": \"agent.root\"}".as_bytes().to_vec(),
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
        "{\"account_id\": \"agent.root\"}".as_bytes(),
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
        "{\"account_id\": \"agent.root\"}".as_bytes(),
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
    // println!("task_view_result {:?}", task_view_result);
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
    // println!("task_view_result {:?}", task_view_result);
    assert!(
        task_view_result.is_ok(),
        "Expected to find hash of task just added."
    );
    let returned_task: TaskHumanFriendly = task_view_result.unwrap_json();

    let expected_task = TaskHumanFriendly {
        owner_id: COUNTER_ID.to_string(),
        contract_id: COUNTER_ID.to_string(),
        function_id: "increment".to_string(),
        cadence: "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2".to_string(),
        recurring: true,
        total_deposit: U128::from(2_600_000_024_000_000_000_000u128),
        deposit: U128::from(12000000000000),
        gas: 3000000000000,
        arguments: Base64VecU8::from(vec![]),
        hash: Base64VecU8::from(vec![
            4, 23, 43, 212, 103, 88, 226, 36, 140, 121, 177, 90, 190, 238, 242, 207, 135, 90, 60,
            48, 235, 150, 108, 83, 127, 151, 237, 11, 68, 65, 242, 100,
        ]),
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
        &json!({ "payable_account_id": NEW_NAME_ID })
            .to_string()
            .into_bytes(),
        DEFAULT_GAS,
        1, // deposit 1 yocto
    );

    let agent_result: Agent = root
        .view(
            cron.account_id(),
            "get_agent",
            &json!({
                "account_id": agent.account_id
            })
            .to_string()
            .into_bytes(),
        )
        .unwrap_json();

    assert_eq!(agent_result.payable_account_id, NEW_NAME_ID);
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
    // Slot is 1860000000000

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
    // Move forward proper amount until a slot where the timestamp becomes valid
    while root_runtime.produce_blocks(1).is_ok() {
        if root_runtime.cur_block.block_timestamp >= 1860000000000 {
            break;
        }
    }

    // Agent calls proxy_call using new transaction syntax with borrowed,
    // mutable runtime object.
    let res = root_runtime.resolve_tx(SignedTransaction::call(
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
    find_log_from_outcomes(&root_runtime, &"Withdrawal of".to_string());
}

#[test]
fn simulate_sputnikv2_interaction() {
    let (root, cron) = sim_helper_init();
    let dao_user = root.create_user(USER_ID.into(), to_yocto("100"));
    let sputnik = sim_helper_init_sputnikv2(&root);

    // tell cron that the DAO is the new owner
    cron.call(
        cron.account_id(),
        "update_settings",
        &json!({
            "owner_id": sputnik.account_id
        })
        .to_string()
        .into_bytes(),
        DEFAULT_GAS,
        0,
    )
    .assert_success();

    // View the agent fee
    let mut agent_info_result = cron.view(
        cron.account_id(),
        "get_info",
        &json!({}).to_string().into_bytes(),
    );

    let mut agent_info: (
        bool,
        AccountId,
        U64,
        U64,
        [u16; 2],
        U128,
        U64,
        U64,
        U128,
        U128,
        U128, // agent fee
        U128,
        U64,
        U64,
        U64,
        U128,
    ) = agent_info_result.unwrap_json();
    let original_agent_fee = agent_info.10;

    // dao user creates a proposal to increase agent fee
    let args = Base64VecU8(
        json!({"agent_fee": "1111111111111111111111",})
            .to_string()
            .into_bytes(),
    );
    dao_user
        .call(
            sputnik.account_id.clone(),
            "add_proposal",
            &json!({
                "proposal": {
                    "description": "increase cron agent fee",
                    "kind": {
                        "FunctionCall": {
                            "receiver_id": cron.account_id,
                            "actions": [{
                                "method_name": "update_settings",
                                "args": args,
                                "deposit": "0",
                                "gas": "100000000000000"
                            }]
                        }
                    }
                }
            })
            .to_string()
            .into_bytes(),
            DEFAULT_GAS,
            10u128.pow(24),
        )
        .assert_success();

    // The dao user approves the proposal
    dao_user
        .call(
            sputnik.account_id.clone(),
            "act_proposal",
            &json!({
                "id": 0,
                "action": "VoteApprove"
            })
            .to_string()
            .into_bytes(),
            DEFAULT_GAS,
            0,
        )
        .assert_success();

    agent_info_result = cron.view(
        cron.account_id(),
        "get_info",
        &json!({}).to_string().into_bytes(),
    );
    agent_info = agent_info_result.unwrap_json();
    let updated_agent_fee = agent_info.10;
    assert_ne!(
        original_agent_fee, updated_agent_fee,
        "Agent fee should have updated"
    );
    assert_eq!(original_agent_fee, U128(500000000000000000000));
    assert_eq!(updated_agent_fee, U128(1111111111111111111111));

    // Ensure that original owner shouldn't be able to call since it's updated
    let expected_failure = cron.call(
        cron.account_id(),
        "update_settings",
        &json!({
            "owner_id": cron.account_id()
        })
        .to_string()
        .into_bytes(),
        DEFAULT_GAS,
        0,
    );
    let status = expected_failure.status();
    match status {
        ExecutionStatus::Failure(f) => {
            // Not great to use `contains` but will have to do for now.
            assert!(
                f.to_string().contains("Must be owner"),
                "Should not be able to call update_settings if not the updated owner"
            );
        }
        _ => panic!("Expected failure after original owner is no longer in control"),
    }
}

#[test]
fn common_tick_workflow() {
    /*
    #- clear, create & bootstrap
    #- register a new agent "agent.ion.testnet"
    #- create more tasks (minimum 4 total)
    #- tick method
    #- some agent tries to call proxy_call and fails
     */
    let (agent_signer, root_account, agent, counter, cron) = bootstrap_time_simulation();

    counter_create_task(&counter, cron.account_id(), "0 3 * * * * *").assert_success();

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

    let second_agent =
        root_account.create_user("second-agent.root".parse().unwrap(), to_yocto("100"));
    second_agent
        .call(
            "cron.root".to_string(),
            "register_agent",
            &json!({}).to_string().into_bytes(),
            DEFAULT_GAS,
            AGENT_REGISTRATION_COST,
        )
        .assert_success();

    // Add a few more tasks
    counter_create_task(&counter, cron.account_id(), "0 13 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "6 19 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "6 31 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "0 47 * * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "0 7 5 * * * *").assert_success();
    counter_create_task(&counter, cron.account_id(), "0 43 * * * * *").assert_success();

    let mut root_runtime = root_account.borrow_runtime_mut();
    assert!(
        root_runtime.produce_blocks(1900).is_ok(),
        "Couldn't produce blocks"
    );

    // Call tick
    let mut res = root_runtime.resolve_tx(SignedTransaction::call(
        2,
        "agent.root".to_string(),
        "cron.root".to_string(),
        &agent_signer.clone(),
        0,
        "tick".into(),
        "{}".as_bytes().to_vec(),
        DEFAULT_GAS,
        CryptoHash::default(),
    ));
    let (_, res_outcome) = res.unwrap();
    assert_eq!(res_outcome.status, ExecutionStatus::SuccessValue(vec![]));

    // Not sure if we need this
    assert!(
        root_runtime.produce_blocks(1900).is_ok(),
        "Couldn't produce blocks"
    );

    // Agent calls proxy_call using new transaction syntax with borrowed,
    // mutable runtime object.
    res = root_runtime.resolve_tx(SignedTransaction::call(
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
}

use manager::{Agent, Task, TaskStatus};
use near_primitives_core::account::Account as PrimitiveAccount;
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json;
use near_sdk::serde_json::json;
use near_sdk_sim::account::AccessKey;
use near_sdk_sim::hash::CryptoHash;
use near_sdk_sim::near_crypto::{InMemorySigner, KeyType, Signer};
use near_sdk_sim::runtime::{GenesisConfig, RuntimeStandalone};
use near_sdk_sim::state_record::StateRecord;
use near_sdk_sim::transaction::{ExecutionStatus, SignedTransaction};
use near_sdk_sim::{init_simulator, to_yocto, UserAccount, DEFAULT_GAS, STORAGE_AMOUNT, ExecutionResult};
use std::cell::RefCell;
use std::rc::Rc;
use near_sdk_sim::types::AccountId;

// Load in contract bytes at runtime
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    CRON_MANAGER_WASM_BYTES => "../res/manager.wasm",
    COUNTER_WASM_BYTES => "../res/rust_counter_tutorial.wasm",
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
    slot: U128
}

fn helper_create_task(cron: &UserAccount, counter: &UserAccount) -> TaskBase64Hash {
    let execution_result = counter.call(
        cron.account_id(),
        "create_task",
        &json!({
            "contract_id": COUNTER_ID,
            "function_id": "increment".to_string(),
            "cadence": "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2".to_string(),
            "recurring": true,
            "deposit": "12000000000000",
            "gas": 3000000000000u64,
        })
        .to_string()
        .into_bytes(),
        DEFAULT_GAS,
        36_000_000_000_000u128, // deposit
    );
    execution_result.assert_success();
    let hash: Base64VecU8 = execution_result.unwrap_json();
    serde_json::to_string(&hash).unwrap()
}

/// Basic initialization returning the "root account" for the simulator
/// and the NFT account with the contract deployed and initialized.
fn sim_helper_init() -> (UserAccount, UserAccount) {
    let mut root_account = init_simulator(None);
    root_account = root_account.create_user("sim".to_string(), to_yocto("1000000"));

    // Deploy cron manager and call "new" method
    let cron = root_account.deploy(&CRON_MANAGER_WASM_BYTES, MANAGER_ID.into(), STORAGE_AMOUNT);
    cron.call(
        cron.account_id(),
        "new",
        &[],
        DEFAULT_GAS,
        0, // attached deposit
    )
    .assert_success();

    (root_account, cron)
}

fn sim_helper_create_agent_user(root_account: &UserAccount) -> (UserAccount, UserAccount) {
    let hundred_near = to_yocto("100");
    let agent = root_account.create_user(AGENT_ID.into(), hundred_near);
    let user = root_account.create_user(USER_ID.into(), hundred_near);
    (agent, user)
}

fn sim_helper_init_counter(root_account: &UserAccount) -> UserAccount {
    // Deploy counter and call "new" method
    let counter = root_account.deploy(&COUNTER_WASM_BYTES, COUNTER_ID.into(), STORAGE_AMOUNT);
    counter
}

fn counter_create_task(counter: &UserAccount, cron: AccountId, cadence: &str) -> ExecutionResult {
    counter.call(
        cron,
        "create_task",
        &json!({
            "contract_id": counter.account_id,
            "function_id": "increment".to_string(),
            "cadence": cadence,
            "recurring": true,
            "deposit": "0",
            // "gas": 100_000_000_000_000u64,
            "gas": 2_400_000_000_000u64,
        })
            .to_string()
            .into_bytes(),
        DEFAULT_GAS,
        120000000200000000000000, // deposit (0.120000000002 Ⓝ)
    )
}

// Begin tests

#[test]
fn simulate_task_creation() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);
}

/// Creates 11 tasks that will occupy different slots, have agent execute them.
#[test]
fn simulate_many_tasks() {
    let mut genesis = GenesisConfig::default();
    let root_account_id = "root".to_string();
    let signer = genesis.init_root_signer(&root_account_id);

    // Make agent signer
    let agent_signer = InMemorySigner::from_seed("agent.root", KeyType::ED25519, "aloha");
    // Push agent account to state_records
    genesis.state_records.push(StateRecord::Account {
        account_id: "agent.root".to_string(),
        account: PrimitiveAccount {
            amount: to_yocto("6000"),
            locked: 0,
            code_hash: Default::default(),
            storage_usage: 0,
        },
    });
    genesis.state_records.push(StateRecord::AccessKey {
        account_id: "agent.root".to_string(),
        public_key: agent_signer.clone().public_key(),
        access_key: AccessKey::full_access(),
    });

    let runtime = RuntimeStandalone::new_with_store(genesis);
    let runtime_rc = &Rc::new(RefCell::new(runtime));
    let root_account = UserAccount::new(runtime_rc, root_account_id, signer);

    // create "counter" account and deploy
    let counter = root_account.deploy(
        &COUNTER_WASM_BYTES,
        "counter.root".to_string(),
        STORAGE_AMOUNT,
    );

    // create "agent" account from signer
    let agent = UserAccount::new(runtime_rc, "agent.root".to_string(), agent_signer.clone());

    // create "cron" account, deploy and call "new"
    let cron = root_account.deploy(
        &CRON_MANAGER_WASM_BYTES,
        "cron.root".to_string(),
        STORAGE_AMOUNT,
    );
    cron.call(
        cron.account_id(),
        "new",
        &[],
        DEFAULT_GAS,
        0, // attached deposit
    )
        .assert_success();

    // Increase agent fee a bit
    cron.call(
        cron.account_id(),
        "update_settings",
        &json!({
            "agent_fee": U128::from(60_000_000_000_000_000_000_000u128)
        })
            .to_string()
            .into_bytes(), // 0.06 Ⓝ
        DEFAULT_GAS,
        0, // attached deposit
    ).assert_success();

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
    agent.call(
        "cron.root".to_string(),
        "register_agent",
        &json!({}).to_string().into_bytes(),
        DEFAULT_GAS,
        AGENT_REGISTRATION_COST,
    ).assert_success();

    // Here's where things get interesting. We must borrow mutable runtime
    // in order to move blocks forward. But once we do, future calls will
    // look different.
    let mut root_runtime = root_account.borrow_runtime_mut();
    assert!(root_runtime.produce_blocks(120).is_ok(), "Couldn't produce blocks");

    // Should find a task
    let mut get_tasks_view_res = root_runtime.view_method_call("cron.root", "get_tasks", "{}".as_bytes());
    let mut success_val = r#"
        [["/YD9yxy6pZjlvra3qkvybKdodL3alsfvR6S62/FiYow="],"120"]
    "#;
    let mut success_vec: Vec<u8> = success_val.trim().into(); // trim because of multiline assignment above
    assert_eq!(get_tasks_view_res.unwrap(), success_vec, "Should find one particular task hash at slot 120");

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

    // Ensure it doesn't find tasks now, except for the same one that's now completed
    get_tasks_view_res = root_runtime.view_method_call("cron.root", "get_tasks", "{}".as_bytes());
    success_val = r#"
        [[],"120"]
    "#;
    success_vec = success_val.trim().into();
    assert_eq!(get_tasks_view_res.unwrap(), success_vec, "Should find no task hashes at slot 120 anymore");

    let mut tasks_info: GetTasksReturn = get_tasks_view_res.unwrap_json();
    assert_eq!(tasks_info.hashes.len(), 0, "Expected no tasks as before");

    success_vec = success_val.trim().into();
    // let sheesh: Vec<u8> = new_success_val.trim().into();
    assert_eq!(get_tasks_view_res.unwrap(), success_vec, "There should not be any tasks at current slot of 120");
    // assert!(root_runtime.produce_blocks(19).is_ok(), "Couldn't produce blocks");
    //
    // // Get tasks will return tasks for next slot; should be empty. 240 is the next slot with tasks
    // get_tasks_view_res = root_runtime.view_method_call("cron.root", "get_tasks", "{}".as_bytes());
    // success_val = r#"
    //     [[],"180"]
    // "#;
    // success_vec = success_val.trim().into();
    // assert_eq!(get_tasks_view_res.unwrap(), success_vec, "Expected no tasks at slot 180");

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
    let (_, res_outcome) = res.unwrap()
        ;
    // Ensure that it panics with a message we expect.
    match res_outcome.status {
        ExecutionStatus::Failure(f) => {
            // Not great to use `contains` but will have to do for now.
            assert!(f.to_string().contains("No tasks available"), "Should have error that no tasks are available");
        },
        _ => panic!("Expected failure when proxy_call has no tasks to execute")
    }

    // Go through the remainder of the slots, executing tasks
    let mut nonce = 4;
    for n in &[240, 360, 600, 720, 1080, 1800, 2520, 2760, 10740, 18360] {
        // produce blocks until next slot
        let cur_block_height = root_runtime.cur_block.block_height;
        assert!(root_runtime.produce_blocks(n - cur_block_height).is_ok(), "Couldn't produce blocks");
        get_tasks_view_res = root_runtime.view_method_call("cron.root", "get_tasks", "{}".as_bytes());
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
        assert_eq!(res_outcome.status, ExecutionStatus::SuccessValue(vec![]), "Expected proxy_call to succeed when looping through.");
    }

    let agent_info_result = root_runtime.view_method_call("cron.root", "get_agent", "{\"account\": \"agent.root\"}".as_bytes());
    let agent_info: Agent = agent_info_result.unwrap_json();
    // Confirm that the agent has executed 11 tasks
    assert_eq!(agent_info.total_tasks_executed.0, 11, "Expected agent to have completed 11 tasks.")
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
        status: TaskStatus::Ready,
        total_deposit: U128::from(36000000000000),
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
    let mut genesis = GenesisConfig::default();
    let root_account_id = "root".to_string();
    let signer = genesis.init_root_signer(&root_account_id);

    // Make agent signer
    let agent_signer = InMemorySigner::from_seed("agent.root", KeyType::ED25519, "aloha");
    // Push agent account to state_records
    genesis.state_records.push(StateRecord::Account {
        account_id: "agent.root".to_string(),
        account: PrimitiveAccount {
            amount: to_yocto("6000"),
            locked: 0,
            code_hash: Default::default(),
            storage_usage: 0,
        },
    });
    genesis.state_records.push(StateRecord::AccessKey {
        account_id: "agent.root".to_string(),
        public_key: agent_signer.clone().public_key(),
        access_key: AccessKey::full_access(),
    });

    let runtime = RuntimeStandalone::new_with_store(genesis);
    let runtime_rc = &Rc::new(RefCell::new(runtime));
    let root_account = UserAccount::new(runtime_rc, root_account_id, signer);

    // create "counter" account and deploy
    let counter = root_account.deploy(
        &COUNTER_WASM_BYTES,
        "counter.root".to_string(),
        STORAGE_AMOUNT,
    );

    // create "agent" account from signer
    let agent = UserAccount::new(runtime_rc, "agent.root".to_string(), agent_signer.clone());

    // create "cron" account, deploy and call "new"
    let cron = root_account.deploy(
        &CRON_MANAGER_WASM_BYTES,
        "cron.root".to_string(),
        STORAGE_AMOUNT,
    );
    cron.call(
        cron.account_id(),
        "new",
        &[],
        DEFAULT_GAS,
        0, // attached deposit
    )
    .assert_success();

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
    agent.call(
        "cron.root".to_string(),
        "register_agent",
        &json!({}).to_string().into_bytes(),
        DEFAULT_GAS,
        AGENT_REGISTRATION_COST,
    ).assert_success();

    // Here's where things get interesting. We must borrow mutable runtime
    // in order to move blocks forward. But once we do, future calls will
    // look different.
    let mut root_runtime = root_account.borrow_runtime_mut();
    // let mut cron_runtime = cron.borrow_runtime();
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
    assert_eq!(new_agent_balance.0, AGENT_REGISTRATION_COST + AGENT_FEE);

    // Agent withdraws balance, claiming rewards
    // Here we don't resolve the transaction, but instead just send it so we can view
    // the receipts generated
    res = root_runtime.resolve_tx(SignedTransaction::call(
        9,
        "agent.root".to_string(),
        "cron.root".to_string(),
        &agent_signer,
        0,
        "withdraw_task_balance".into(),
        "{}".as_bytes().to_vec(),
        DEFAULT_GAS,
        CryptoHash::default(),
    ));

    let (_, res_outcome) = res.unwrap();
    root_runtime.process_all().unwrap();
    let last_outcomes = &root_runtime.last_outcomes;

    // This isn't great, but we check to make sure the log exists about the transfer
    // At the time of this writing, finding the TransferAction with the correct
    // deposit was not happening with simulation tests.
    // Look for a log saying "Withdrawal of 60000000000000000000000 has been sent." in one of these
    let mut found_withdrawal_log = false;
    for outcome_hash in last_outcomes {
        let eo = root_runtime.outcome(&outcome_hash).unwrap();
        if eo.logs.contains(&"Withdrawal of 60000000000000000000000 has been sent.".to_string()) {
            found_withdrawal_log = true;
        }
    }
    assert!(found_withdrawal_log, "Expected a recent outcome to have a log about the transfer action.");

    // let logs = res_outcome.logs;
    // assert_eq!(logs.len(), 1, "Expected one log after agent withdrawal.");
    // assert_eq!(logs[0].as_str(), "Withdrawal of 60000000000000000000000 has been sent.");
}

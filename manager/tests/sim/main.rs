use near_sdk::json_types::{U128, U64, Base64VecU8};
use near_sdk::serde_json::json;
use near_sdk::{Balance, serde_json};
use near_sdk_sim::transaction::ExecutionStatus;
use near_sdk_sim::{init_simulator, to_yocto, UserAccount, DEFAULT_GAS, STORAGE_AMOUNT, runtime::init_runtime};
use manager::{CronManager, Task, TaskStatus};
use near_sdk_sim::types::AccountId;
use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use near_sdk_sim::runtime::RuntimeStandalone;

// Load in contract bytes at runtime
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    CRON_MANAGER_WASM_BYTES => "../res/manager.wasm",
    COUNTER_WASM_BYTES => "../res/rust_counter_tutorial.wasm",
}

const MANAGER_ID: &str = "manager.sim";
const COUNTER_ID: &str = "counter.sim";
const TASK_BASE64: &str = "chUCZxP6uO5xZIjwI9XagXVUCV7nmE09HVRUap8qauo=";

type TaskBase64Hash = String;

fn helper_create_task(cron: &UserAccount, counter: &UserAccount) -> TaskBase64Hash {
    let execution_result = counter.call(
        cron.account_id(),
        "create_task",
        &json!({
            "contract_id": COUNTER_ID,
            "function_id": "increment".to_string(),
            "cadence": "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2".to_string(),
            "recurring": true,
            "deposit": 0,
            "gas": 3000000000000u64,
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        6_000_000_000_000u128, // deposit
    );
    execution_result.assert_success();
    let hash: Base64VecU8 = execution_result.unwrap_json();
    serde_json::to_string(&hash).unwrap()
}

fn helper_next_epoch(runtime: &mut RuntimeStandalone) {
    let epoch_height = runtime.current_block().epoch_height;
    while epoch_height == runtime.current_block().epoch_height {
        runtime.produce_block().unwrap();
    }
}

#[test]
fn simulate_task_creation() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);
}

#[test]
fn simulate_next_epoch() {
    // TODO: fill this test out, this is just the basics of moving blocks forward.
    let (mut runtime, signer, root_account_id) = init_runtime(None);
    let root_account = UserAccount::new(&Rc::new(RefCell::new(runtime)), root_account_id, signer);

    let mut root_runtime = root_account.borrow_runtime_mut();
    let block_production_result = root_runtime.produce_blocks(7);
    assert!(block_production_result.is_ok(), "Couldn't produce blocks");
    println!("aloha current block height {}, epoch height {}", root_runtime.current_block().block_height, root_runtime.current_block().epoch_height);
}

#[test]
fn simulate_basic_task_checks() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);

    // Nonexistent task fails.
    let mut task_view_result = root
        .view(
            cron.account_id(),
            "get_task",
            &json!({
                "task_hash": "doesnotexist"
            }).to_string().into_bytes(),
        );
    assert!(task_view_result.is_err(), "Expected nonexistent task to throw error.");
    let error = task_view_result.unwrap_err();
    let error_message = error.to_string();
    assert!(error_message.contains("No task found by hash"));

    // Get has from task just added.
    task_view_result = root
        .view(
            cron.account_id(),
            "get_task",
            &json!({
                "task_hash": TASK_BASE64
            }).to_string().into_bytes(),
        );
    assert!(task_view_result.is_ok(), "Expected to find hash of task just added.");
    let returned_task: Task = task_view_result.unwrap_json();

    let expected_task = Task {
        owner_id: COUNTER_ID.to_string(),
        contract_id: COUNTER_ID.to_string(),
        function_id: "increment".to_string(),
        cadence: "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2".to_string(),
        recurring: true,
        status: TaskStatus::Ready,
        total_deposit: 6000000000000,
        deposit: 0,
        gas: 3000000000000,
        arguments: vec![]
    };
    assert_eq!(expected_task, returned_task, "Task returned was not expected.");

    // Attempt to remove task with non-owner account.
    let mut removal_result = root.call(
        cron.account_id(),
        "remove_task",
        &json!({
            "task_hash": TASK_BASE64
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        0, // deposit
    );
    let status = removal_result.status();
    if let ExecutionStatus::Failure(err) = status {
        // At this time, this is the way to check for error messages.
        assert!(err.to_string().contains("Only owner can remove their task."));
    } else {
        panic!("Non-owner account should not succeed in removing task.");
    }

    counter.call(
        cron.account_id(),
        "remove_task",
        &json!({
            "task_hash": TASK_BASE64
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        0, // deposit
    ).assert_success();

    // Get has from task just added.
    task_view_result = root
        .view(
            cron.account_id(),
            "get_task",
            &json!({
                "task_hash": TASK_BASE64
            }).to_string().into_bytes(),
        );
    assert!(task_view_result.is_err(), "Expected error when trying to retrieve removed task.");
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
    let agent = root_account.create_user("agent.sim".to_string(), hundred_near);
    let user = root_account.create_user("user.sim".to_string(), hundred_near);
    (agent, user)
}

fn sim_helper_init_counter(root_account: &UserAccount) -> UserAccount {
    // Deploy counter and call "new" method
    let counter = root_account.deploy(&COUNTER_WASM_BYTES, COUNTER_ID.into(), STORAGE_AMOUNT);
    counter
}

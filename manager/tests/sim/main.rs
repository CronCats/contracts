use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde_json::json;
use near_sdk::serde_json;
use near_sdk_sim::transaction::{ExecutionStatus, SignedTransaction};
use near_sdk_sim::{init_simulator, to_yocto, UserAccount, DEFAULT_GAS, STORAGE_AMOUNT};
use manager::{Task, TaskStatus, Agent};
use std::rc::Rc;
use std::cell::RefCell;
use near_sdk_sim::runtime::{RuntimeStandalone, GenesisConfig};
use near_sdk_sim::hash::CryptoHash;
use near_sdk_sim::near_crypto::{InMemorySigner, KeyType, Signer};
use near_sdk_sim::state_record::StateRecord;
use near_sdk_sim::account::AccessKey;
use near_primitives_core::account::Account as PrimitiveAccount;

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
            "deposit": "12000000000000",
            "gas": 3000000000000u64,
        }).to_string().into_bytes(),
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

// Begin tests

#[test]
fn simulate_task_creation() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);
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
        total_deposit: U128::from(36000000000000),
        deposit: U128::from(12000000000000),
        gas: 3000000000000,
        arguments: vec![]
    };
    assert_eq!(expected_task, returned_task, "Task returned was not expected.");

    // Attempt to remove task with non-owner account.
    let removal_result = root.call(
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

    // Get hash from task just removed.
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

#[test]
fn simulate_basic_agent_registration_update() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);
    let (agent, _) = sim_helper_create_agent_user(&root);

    // Register an agent, where the beneficiary is user.sim
    agent.call(
        cron.account_id(),
        "register_agent",
        &json!({
            "payable_account_id": USER_ID
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        AGENT_REGISTRATION_COST, // deposit
    ).assert_success();

    // Attempt to re-register
    let mut failed_result = agent.call(
        cron.account_id(),
        "register_agent",
        &json!({
            "payable_account_id": USER_ID
        }).to_string().into_bytes(),
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
        }).to_string().into_bytes(),
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
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        0, // deposit
    );

    let agent_result: Agent = root.view(
        cron.account_id(),
        "get_agent",
        &json!({
            "account": agent.account_id
        }).to_string().into_bytes(),
    ).unwrap_json();

    assert_eq!(agent_result.payable_account_id, "newname.sim".to_string());
}

#[test]
fn simulate_agent_unregister_check() {
    let (root, cron) = sim_helper_init();
    let counter = sim_helper_init_counter(&root);
    helper_create_task(&cron, &counter);
    let unregister_result = cron.call(
        cron.account_id(),
        "unregister_agent",
        &[],
        DEFAULT_GAS,
        1
    );
    unregister_result.assert_success();
    for log in unregister_result.logs() {
        assert_eq!(log, &"The agent manager.sim is not registered".to_string());
    }
}

#[test]
fn simulate_task_creation_agent_usage() {
    let mut genesis = GenesisConfig::default();
    genesis.runtime_config.wasm_config.limit_config.max_total_prepaid_gas = genesis.gas_limit;
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
            storage_usage: 0
        }
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
    let counter = root_account.deploy(&COUNTER_WASM_BYTES, "counter.root".to_string(), STORAGE_AMOUNT);

    // create "agent" account from signer
    let agent  = UserAccount::new(runtime_rc, "agent.root".to_string(), agent_signer.clone());

    // create "cron" account, deploy and call "new"
    let cron = root_account.deploy(&CRON_MANAGER_WASM_BYTES, "cron.root".to_string(), STORAGE_AMOUNT);
    cron.call(
        cron.account_id(),
        "new",
        &[],
        DEFAULT_GAS,
        0, // attached deposit
    ).assert_success();

    // Increase agent fee a bit
    cron.call(
        cron.account_id(),
        "update_settings",
        &json!({
            "agent_fee": U128::from(60_000_000_000_000_000_000_000u128)
        }).to_string().into_bytes(), // 0.06 Ⓝ
        DEFAULT_GAS,
        0, // attached deposit
    ).assert_success();

    // create a task
    let execution_result = counter.call(
        cron.account_id(),
        "create_task",
        &json!({
            "contract_id": COUNTER_ID,
            "function_id": "increment".to_string(),
            "cadence": "0 30 * * * * *".to_string(), // every hour at 30 min
            "recurring": true,
            "deposit": "0",
            "gas": 1000000000000u64,
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        120000000002000000000000u128, // deposit (0.120000000002 Ⓝ)
    );
    execution_result.assert_success();

    // register agent
    agent.call("cron.root".to_string(), "register_agent", &json!({}).to_string().into_bytes(), DEFAULT_GAS, 2090000000000000000000).assert_success();

    // Here's where things get interesting. We must borrow mutable runtime
    // in order to move blocks forward. But once we do, future calls will
    // look different.
    let mut root_runtime = root_account.borrow_runtime_mut();
    // Move forward proper amount until slot 1740
    let block_production_result = root_runtime.produce_blocks(1780);
    assert!(block_production_result.is_ok(), "Couldn't produce blocks");
    println!("Current block height {}, epoch height {}", root_runtime.current_block().block_height, root_runtime.current_block().epoch_height);
    println!("Current timestamp {}", root_runtime.current_block().block_timestamp);

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
    let (_, res) = res.unwrap();
    assert_eq!(res.status, ExecutionStatus::SuccessValue(vec![]));
    root_runtime.process_all().unwrap();

    // Check agent's balance after proxy call
    let mut agent_view = root_runtime.view_account("agent.root").unwrap();
    let mut agent_amount = agent_view.amount;
    println!("agent_amount after proxy call \t\t{:?}", agent_amount);

    // Agent withdraws balance, claiming rewards
    let res = root_runtime.resolve_tx(SignedTransaction::call(
        7,
        "agent.root".to_string(),
        "cron.root".to_string(),
        &agent_signer,
        0,
        "withdraw_task_balance".into(),
        "{}".as_bytes().to_vec(),
        DEFAULT_GAS,
        CryptoHash::default(),
    ));
    let (_, res) = res.unwrap();
    // res.
    assert_eq!(res.status, ExecutionStatus::SuccessValue(vec![]));
    root_runtime.process_all().expect("Issue withdrawing task balance");

    // Check agent's balance after withdrawal
    agent_view = root_runtime.view_account("agent.root").unwrap();
    agent_amount = agent_view.amount;
    println!("agent_amount after agent withdraw \t{:?}", agent_amount);
}

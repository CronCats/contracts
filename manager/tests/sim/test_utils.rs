use crate::{TaskBase64Hash, AGENT_ID, COUNTER_ID, COUNTER_WASM_BYTES, CRON_MANAGER_WASM_BYTES, SPUTNIKV2_WASM_BYTES, MANAGER_ID, USER_ID, SPUTNIKV2_ID};
use near_primitives_core::account::Account as PrimitiveAccount;
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde_json;
use near_sdk::serde_json::json;
use near_sdk_sim::account::AccessKey;
use near_sdk_sim::near_crypto::{InMemorySigner, KeyType, Signer};
use near_sdk_sim::runtime::{GenesisConfig, RuntimeStandalone};
use near_sdk_sim::state_record::StateRecord;
use near_sdk_sim::types::AccountId;
use near_sdk_sim::{
    init_simulator, to_yocto, ExecutionResult, UserAccount, DEFAULT_GAS, STORAGE_AMOUNT,
};
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

pub(crate) fn helper_create_task(cron: &UserAccount, counter: &UserAccount) -> TaskBase64Hash {
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
        2_600_000_024_000_000_000_000u128, // deposit
    );
    execution_result.assert_success();
    let hash: Base64VecU8 = execution_result.unwrap_json();
    serde_json::to_string(&hash).unwrap()
}

/// Basic initialization returning the "root account" for the simulator
/// and the NFT account with the contract deployed and initialized.
pub(crate) fn sim_helper_init() -> (UserAccount, UserAccount) {
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

pub(crate) fn sim_helper_create_agent_user(
    root_account: &UserAccount,
) -> (UserAccount, UserAccount) {
    let hundred_near = to_yocto("100");
    let agent = root_account.create_user(AGENT_ID.into(), hundred_near);
    let user = root_account.create_user(USER_ID.into(), hundred_near);
    (agent, user)
}

pub(crate) fn sim_helper_init_counter(root_account: &UserAccount) -> UserAccount {
    // Deploy counter
    let counter = root_account.deploy(&COUNTER_WASM_BYTES, COUNTER_ID.into(), STORAGE_AMOUNT);
    counter
}

pub(crate) fn sim_helper_init_sputnikv2(root_account: &UserAccount) -> UserAccount {
    // Deploy SputnikDAOv2 and call "new" method
    let sputnik = root_account.deploy(&SPUTNIKV2_WASM_BYTES, SPUTNIKV2_ID.into(), STORAGE_AMOUNT);
    /*
    export COUNCIL='["'$CONTRACT_ID'"]'
    near call $CONTRACT_ID new '{"config": {"name": "genesis2", "purpose": "test", "metadata": ""}, "policy": '$COUNCIL'}' --accountId $CONTRACT_ID
     */
    root_account.call(
        sputnik.account_id.clone(),
        "new",
        &json!({
            "config": {
                "name": "cron dao",
                "purpose": "not chew bubble gum",
                "metadata": ""
            },
            "policy": [USER_ID]
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        0
    );
    sputnik
}

pub(crate) fn counter_create_task(
    counter: &UserAccount,
    cron: AccountId,
    cadence: &str,
) -> ExecutionResult {
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
        120480000000000000000000, // deposit (0.120000000002 Ⓝ)
    )
}

pub(crate) fn bootstrap_time_simulation() -> (
    InMemorySigner,
    UserAccount,
    UserAccount,
    UserAccount,
    UserAccount,
) {
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

    (agent_signer, root_account, agent, counter, cron)
}

pub(crate) fn find_log_from_outcomes(root_runtime: &RefMut<RuntimeStandalone>, msg: &String) {
    let last_outcomes = &root_runtime.last_outcomes;

    // This isn't great, but we check to make sure the log exists about the transfer
    // At the time of this writing, finding the TransferAction with the correct
    // deposit was not happening with simulation tests.
    // Look for a log saying "Withdrawal of 60000000000000000000000 has been sent." in one of these
    let mut found_withdrawal_log = false;
    for outcome_hash in last_outcomes {
        let eo = root_runtime.outcome(&outcome_hash).unwrap();
        for log in eo.logs {
            if log.contains(msg) {
                found_withdrawal_log = true;
            }
        }
    }
    assert!(
        found_withdrawal_log,
        "Expected a recent outcome to have a log about the transfer action. Log: {}",
        msg
    );
}

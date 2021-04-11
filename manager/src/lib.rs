use near_sdk::{
    near_bindgen,
    log,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LookupMap, TreeMap},
    json_types::{ValidAccountId, Base58PublicKey},
    serde_json::json,
    AccountId,
    Balance,
    env,
    Promise,
    PublicKey,
    PanicOnDefault
};

near_sdk::setup_alloc!();

// Balance & Fee Definitions
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const GAS_BASE_FEE: u64 = 3_000_000_000_000;
pub const STAKE_BALANCE_MIN: u128 = 10 * ONE_NEAR;

// Boundary Definitions
pub const MAX_BLOCK_RANGE: u32 = 1_000_000;
pub const MAX_EPOCH_RANGE: u32 = 10_000;
pub const MAX_SECOND_RANGE: u32 = 600_000_000;
pub const SLOT_GRANULARITY: u64 = 100;

/// Allows tasks to be executed in async env
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum TaskStatus {
    /// Shows a task is not currently active, ready for an agent to take
    Ready,

    /// Shows a task is currently being processed/called
    Active,

    /// Tasks marked as complete are ready for deletion from state. 
    Complete
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Task {
    /// Entity responsible for this task, can change task details
    owner_id: AccountId,

    /// Account to direct all execution calls against
    contract_id: AccountId,

    /// Contract method this task will be executing
    function_id: String,

    /// Crontab + Consensustab Spec String
    /// Defines the interval spacing of execution
    // TODO: Change to the time parser type
    cadence: String,

    /// Defines if this task can continue until balance runs out
    recurring: bool,

    /// Tasks status forces single executions per interval
    status: TaskStatus,

    /// Total balance of NEAR available for current and future executions
    balance: Balance,

    /// Configuration of NEAR balance to send to each function call. This is the "amount" for a function call.
    fn_deposit: Balance,

    /// Configuration of NEAR balance to attach to each function call. This is the "gas" for a function call.
    gas_allowance: u64,

    // NOTE: Only allow static pre-defined bytes
    arguments: Vec<u8>
}

impl ToString for Task {
    fn to_string(&self) -> String {
        json!({
            "owner_id": self.owner_id,
            "contract_id": self.contract_id,
            "function_id": self.function_id,
            "cadence": self.cadence,
            "recurring": self.recurring.to_string(),
            "status": format!("{:?}", self.status), // FYI, prolly better way to do this
            "balance": self.balance.to_string(),
            "fn_deposit": self.function_id,
            "gas_allowance": self.gas_allowance,
            "arguments": self.arguments,
        }).to_string()
    }
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, Debug)]
pub struct Agent {
    pk: PublicKey,
    account_id: AccountId,
    payable_account_id: AccountId,
    balance: Balance,
    total_tasks_executed: u128
}

impl ToString for Agent {
    fn to_string(&self) -> String {
        json!({
            "payable_account_id": self.payable_account_id,
            "balance": self.balance.to_string(),
            "total_tasks_executed": self.total_tasks_executed.to_string()
        }).to_string()
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CronManager {
    // Runtime
    // TODO: Setup DAO based management & ownership
    paused: bool,
    owner_id: AccountId,
    owner_pk: PublicKey,

    // Basic management
    tasks: LookupMap<Vec<u8>, Task>,
    agents: LookupMap<PublicKey, Agent>,
    slots: TreeMap<u128, Vec<Vec<u8>>>,

    // Economics
    available_balance: Balance,
    staked_balance: Balance,
    agent_fee: Balance,
    slot_granularity: u64
}

#[near_bindgen]
impl CronManager {
    /// ```bash
    /// near call cron.testnet new --accountId cron.testnet
    /// ```
    #[init(ignore_state)]
    #[payable]
    pub fn new() -> Self {
        CronManager {
            paused: false,
            owner_id: env::signer_account_id(),
            owner_pk: env::signer_account_pk(),
            tasks: LookupMap::new(vec![5]),
            agents: LookupMap::new(vec![2]),
            slots: TreeMap::new(vec![6]),
            available_balance: 0,
            staked_balance: 0,
            agent_fee: u128::from(GAS_BASE_FEE),
            slot_granularity: SLOT_GRANULARITY
        }
    }

    /// ```bash
    /// near view cron.testnet get_tasks --accountId YOU.testnet
    /// ```
    /// Gets next set of immediate tasks. Limited to return only next set of available ex
    pub fn get_tasks(&self) -> Vec<Vec<u8>> {
        let current_slot = self.get_slot_id(None);
        let default_vec: Vec<Vec<u8>> = Vec::new();

        // get tasks based on current slot
        self.slots.get(&current_slot).unwrap_or(default_vec)
    }

    /// ```bash
    /// near view cron.testnet get_task '{"task_hash": [0,102,143...]}' --accountId YOU.testnet
    /// ```
    /// Gets the data payload of a single task
    pub fn get_task(&self, task_hash: Vec<u8>) -> String {
        let task = self.tasks.get(&task_hash)
            .expect("No task found by hash");

        task.to_string()
    }

    /// Allows any user or contract to pay for future txns based on a specific schedule
    /// contract, function id & other settings. When the task runs out of balance
    /// the task is no longer executed, any additional funds will be returned to task owner.
    ///
    /// ```bash
    /// near call cron.testnet create_task '{"contract_id": "counter.in.testnet","function_id": "increment","cadence": "@epoch","recurring": true,"fn_deposit": 0,"gas_allowance": 2400000000000}' --accountId YOU.testnet
    /// ```
    #[payable]
    pub fn create_task(
        &mut self,
        contract_id: String,
        function_id: String,
        cadence: String, // TODO: Change to the time parser type
        recurring: Option<bool>,
        fn_deposit: Option<u128>,
        gas_allowance: Option<u64>,
        arguments: Option<Vec<u8>>
    ) -> Vec<u8> {
        // TODO: Add asserts to check cadence can be parsed
        log!("cadence {}", &cadence.clone());
        let item = Task {
            owner_id: env::signer_account_id(),
            contract_id,
            function_id,
            cadence,
            recurring: Some(recurring).unwrap_or(Some(false)).unwrap(),
            status: TaskStatus::Ready,
            balance: env::attached_deposit(),
            fn_deposit: Some(fn_deposit).unwrap_or(Some(0)).unwrap(),
            gas_allowance: Some(gas_allowance).unwrap_or(Some(GAS_BASE_FEE)).unwrap(),
            arguments: Some(arguments).unwrap_or(Some(b"".to_vec())).unwrap()
        };

        // Check that balance is sufficient for 1 execution minimum
        let call_balance_used = self.task_balance_uses(&item);
        assert!(call_balance_used < item.balance, "Not enough task balance to execute job");

        let hash = self.hash(&item);
        log!("Task Hash {:?}", &hash);
        // log!("Task data {:?}", &item.to_string());

        // TODO: Parse cadence, insert in slots where necessary
        // TODO: Change! testing with 200 blocks
        let next_slot = self.get_slot_id(Some(200));

        // Add tast to catalog
        self.tasks.insert(&hash, &item);

        // Get previous task hashes in slot, add as needed
        let default_vec: Vec<Vec<u8>> = Vec::new();
        let mut slot_slots = self.slots.get(&next_slot).unwrap_or(default_vec);
        slot_slots.push(hash.clone());
        self.slots.insert(&next_slot, &slot_slots);

        hash
    }

    /// ```bash
    /// near call cron.testnet update_task '{TBD}' --accountId YOU.testnet
    /// ```
    // #[payable]
    // pub fn update_task(
    //     &mut self,
    //     task_hash: String,
    //     contract_id: AccountId,
    //     cadence: String, // TODO: Change to the time parser type
    //     arguments: String
    // ) -> Task {
    //     // TODO: 
    // }

    // TODO: Finish
    // /// ```bash
    // /// near call cron.testnet remove_task '{"task_hash": ""}' --accountId YOU.testnet
    // /// ```
    // pub fn remove_task(
    //     &mut self,
    //     task_hash: u128,
    // ) -> Option<Vec<u8>> {
    //     // TODO: Add asserts: owner only, 
    //     self.slots.remove(&task_hash)
    // }

    /// Executes a task based on the current task slot
    /// Computes whether a task should continue further or not
    /// Makes a cross-contract call with the task configuration
    /// Called directly by a registered agent
    ///
    /// ```bash
    /// near call cron.testnet proxy_call --accountId YOU.testnet
    /// ```
    // TODO: Change OOP, be careful on rewind execution!
    // TODO: How can this promise execute and allow panics?
    pub fn proxy_call(&mut self) {
        // only registered agent signed, because micropayments will benefit long term
        let mut agent = self.agents.get(&env::signer_account_pk())
            .expect("Agent not registered");

        // Get current slot based on block or timestamp
        let current_slot = self.get_slot_id(None);
        log!("current slot {:?}", current_slot);

        // get task based on current slot
        let mut tab = self.slots.get(&current_slot)
            .expect("No tasks found in slot");
        log!("tab {:?}", &tab);

        // TODO: update state, post pop
        // Get a single task hash, then retrieve task details
        let hash = tab.pop().expect("No tasks available");
        let mut task = self.tasks.get(&hash)
            .expect("No task found by hash");
        log!("Found Task {}", &task.to_string());

        // let hash = self.hash(&task);
        let call_balance_used = self.task_balance_uses(&task);

        assert!(call_balance_used < task.balance, "Not enough task balance to execute job");

        // Increment agent reward & task count
        agent.balance += self.agent_fee;
        agent.total_tasks_executed += 1;

        // TODO: Process task exit, if no future task can execute

        // TODO: finish!
        if task.recurring == true {
            // TODO: Change! testing with 200 blocks
            let next_slot = self.get_slot_id(Some(200));
            assert!(&current_slot < &next_slot, "Cannot execute task in the past");

            // Get previous task hashes in slot, add as needed
            let default_vec: Vec<Vec<u8>> = Vec::new();
            let mut slot_slots = self.slots.get(&next_slot).unwrap_or(default_vec);
            slot_slots.push(hash.clone());
            self.slots.insert(&next_slot, &slot_slots);
        }

        // Update agent storage
        self.agents.insert(&env::signer_account_pk(), &agent);

        // Call external contract with task variables
        env::promise_create(
            task.contract_id.clone(),
            &task.function_id.as_bytes(),
            json!({}).to_string().as_bytes(),
            // NOTE: Does this work if signer sends NO amount? Who pays??
            Some(task.fn_deposit).unwrap_or(0),
            Some(task.gas_allowance).unwrap_or(env::prepaid_gas() - env::used_gas())
        );

        // Decrease task balance
        // TODO: Change to real gas used
        task.balance -= self.task_balance_used();

        // Update task storage
        self.tasks.insert(&hash, &task);
    }

    // TODO: Need to have an "exit" flow for tasks that are out of balance

    /// Keep track of this agent, allows for rewards tracking
    ///
    /// ```bash
    /// near call cron.testnet register_agent '{"payable_account_id": "YOU.testnet"}' --accountId YOUR_AGENT.testnet
    /// ```
    pub fn register_agent(
        &mut self,
        payable_account_id: Option<ValidAccountId>
    ) {
        // check that account isnt already added
        if let Some(a) = self.agents.get(&env::signer_account_pk()) {
            panic!("Agent {} already exists", a.account_id);
        };
        let pk = env::signer_account_pk();
        let payable_id;
        match payable_account_id.clone() {
            Some(_id) => {
                payable_id = payable_account_id.unwrap().to_string();
            }
            None => {
                payable_id = env::signer_account_id();
            }
        }

        let agent = Agent {
            pk: pk.clone(),
            account_id: env::signer_account_id(),
            payable_account_id: payable_id,
            balance: 0,
            total_tasks_executed: 0
        };

        self.agents.insert(&pk.into(), &agent);
    }

    /// ```bash
    /// near call cron.testnet update_agent '{"payable_account_id": "YOU.testnet"}' --accountId YOUR_AGENT.testnet
    /// ```
    pub fn update_agent(
        &mut self,
        payable_account_id: Option<ValidAccountId>
    ) {
        let pk = env::signer_account_pk();

        // check that signer agent exists
        if let Some(mut agent) = self.agents.get(&pk) {
            match payable_account_id.clone() {
                Some(_id) => {
                    agent.payable_account_id = payable_account_id.unwrap().to_string();
                }
                None => ()
            }

            self.agents.insert(&pk.into(), &agent);
        } else {
            panic!("Agent must register");
        };
    }

    /// ```bash
    /// near call cron.testnet unregister_agent --accountId YOUR_AGENT.testnet
    /// ```
    pub fn unregister_agent(&mut self) {
        let pk = env::signer_account_pk();

        // check that signer agent exists
        if let Some(_acct) = self.agents.get(&pk) {
            self.agents.remove(&pk);
        } else {
            panic!("No Agent");
        };
    }

    /// ```bash
    /// near call cron.testnet withdraw_task_balance --accountId YOUR_AGENT.testnet
    /// ```
    pub fn withdraw_task_balance(&mut self) -> Promise {
        let pk = env::signer_account_pk();

        // check that signer agent exists
        if let Some(agent) = self.agents.get(&pk) {
            assert!(agent.balance > 0, "No Agent balance");
            Promise::new(agent.payable_account_id.to_string())
                .transfer(agent.balance)
        } else {
            panic!("No Agent");
        }

    }

    /// Gets the agent data stats
    ///
    /// ```bash
    /// near view cron.testnet get_agent '{"pk": "ed25519:AGENT_PUBLIC_KEY"}' --accountId YOU.testnet
    /// ```
    pub fn get_agent(&self, pk: Base58PublicKey) -> String {
        let agent = self.agents.get(&pk.into())
            .expect("No agent found");

        agent.to_string()
    }

    fn hash(&self, item: &Task) -> Vec<u8> {
        // Generate hash
        let input = format!(
                "{:?}{:?}{:?}",
                // "{:?}{:?}",
                item.contract_id,
                item.function_id,
                item.cadence
            );
        env::keccak256(input.as_bytes())
    }

    /// Returns the base amount required to execute 1 task
    fn task_balance_used(&self) -> u128 {
        u128::from(env::used_gas()) + self.agent_fee
    }

    /// Returns the base amount required to execute 1 task
    fn task_balance_uses(&self, task: &Task) -> u128 {
        task.fn_deposit + u128::from(task.gas_allowance) + self.agent_fee
    }

    // TODO: this will need a major overhaul, for now simplify! (needs to work with timestamps as well)
    /// If no offset, Returns current slot based on current block height
    /// If offset, Returns next slot based on current block height & integer offset
    /// rounded to nearest granularity (~every 60 blocks)
    fn get_slot_id(&self, offset: Option<u64>) -> u128 {
        let block = env::block_index();
        let rem = block % self.slot_granularity;

        match Some(offset).unwrap() {
            Some(o) => u128::from(block - rem + o),
            None => u128::from(block - rem)
        }
    }
}

// #[cfg(all(test, not(target_arch = "wasm32")))]
// mod tests {
//     use near_sdk::test_utils::{accounts, VMContextBuilder};
//     use near_sdk::json_types::{ValidAccountId};
//     use near_sdk::MockedBlockchain;
//     use near_sdk::{testing_env};

//     use super::*;

//     fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
//         let mut builder = VMContextBuilder::new();
//         builder
//             .current_account_id(accounts(0))
//             .signer_account_id(predecessor_account_id.clone())
//             .predecessor_account_id(predecessor_account_id);
//         builder
//     }

//     #[test]
//     fn test_thang() {
//         let mut context = get_context(accounts(1));
//         testing_env!(context.build());
//         let contract = CronManager::new();
//         testing_env!(context.is_view(true).build());
//         assert_eq!(contract.thang(), "hi");
//     }
// }
use near_sdk::{
    near_bindgen,
    log,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LookupMap, TreeMap},
    json_types::{ValidAccountId},
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
pub const GAS_BASE_FEE: u128 = 3_000_000_000_000;
pub const STAKE_BALANCE_MIN: u128 = 10 * ONE_NEAR;

// Boundary Definitions
pub const MAX_BLOCK_RANGE: u32 = 1_000_000;
pub const MAX_EPOCH_RANGE: u32 = 10_000;
pub const MAX_SECOND_RANGE: u32 = 600_000_000;
pub const SLOT_GRANULARITY: u64 = 100;

/// Allows tasks to be executed in async env
#[derive(BorshDeserialize, BorshSerialize)]
pub enum TaskStatus {
    /// Shows a task is not currently active, ready for an agent to take
    Ready,

    /// Shows a task is currently being processed/called
    Active,

    /// Tasks marked as complete are ready for deletion from state. 
    Complete
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
    tabs: TreeMap<u128, Vec<u8>>,

    // Economics
    available_balance: Balance,
    staked_balance: Balance,
    agent_fee: Balance
}

#[derive(BorshDeserialize, BorshSerialize)]
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
    tick: String,

    /// Pre-computed block or timestamp of when a task should be called next.
    /// NOTE: This is TBD, needs PoC testing
    next_tick: String,

    /// Defines if this task can continue until balance runs out
    recurring: bool,

    /// Tasks status forces single executions per interval
    status: TaskStatus,

    /// Total balance of NEAR available for current and future executions
    balance: Balance,

    /// Configuration of NEAR balance to send to each function call. This is the "amount" for a function call.
    fn_allowance: Balance,

    /// Configuration of NEAR balance to send to each function call. This is the "amount" for a function call.
    gas_allowance: Balance

    // TODO: Test if this is "safe"
    // arguments: String
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Agent {
    pk: PublicKey,
    account_id: AccountId,
    payable_account_id: AccountId,
    balance: Balance,
    total_tasks_executed: u128
}

#[near_bindgen]
impl CronManager {
    /// ```bash
    /// near call cron.testnet new --accountId cron.testnet
    /// ```
    #[init]
    #[payable]
    pub fn new() -> Self {
        CronManager {
            paused: false,
            owner_id: env::signer_account_id(),
            owner_pk: env::signer_account_pk(),
            tasks: LookupMap::new(vec![1]),
            agents: LookupMap::new(vec![2]),
            tabs: TreeMap::new(vec![0]),
            available_balance: 0,
            staked_balance: 0,
            agent_fee: GAS_BASE_FEE
        }
    }

    /// ```bash
    /// near view cron.testnet get_tasks --accountId YOU.testnet
    /// ```
    // /// Gets next tick immediate tasks. Limited to return only next set of available ex
    // // TODO: finish
    // pub fn get_tasks(&self) -> UnorderedSet<Task> {
    //     assert_ne!(self.tabs.len(), 0);
    //     self.tabs
    // }

    /// ```bash
    /// near call cron.testnet create_task '{"contract_id": "counter.in.testnet","function_id": "increment","tick": "@epoch","recurring": true,"fn_allowance": 0,"gas_allowance": 2400000000000}' --accountId YOU.testnet
    /// ```
    #[payable]
    pub fn create_task(
        &mut self,
        contract_id: AccountId,
        function_id: String,
        tick: String, // TODO: Change to the time parser type
        recurring: Option<bool>,
        fn_allowance: Option<Balance>,
        gas_allowance: Option<Balance>
    ) -> Vec<u8> {
        // TODO: Add asserts, should check that balance can cover 1 task, and storage for a task
        let item = Task {
            owner_id: env::signer_account_id(),
            contract_id,
            function_id,
            tick: tick.clone(),
            recurring: Some(recurring).unwrap_or(Some(false)).unwrap(),
            status: TaskStatus::Ready,
            balance: env::attached_deposit(),
            fn_allowance: Some(fn_allowance).unwrap_or(Some(0)).unwrap(),
            gas_allowance: Some(gas_allowance).unwrap_or(Some(GAS_BASE_FEE)).unwrap(),
            next_tick: "".to_string()
        };

        log!("tick {}", &tick);
        let hash = self.hash(&item);
        log!("Task Hash {:?}", &hash);

        // Add tast to catalog
        self.tasks.insert(&hash, &item);

        // TODO: Parse tick, insert in tabs where necessary
        self.tabs.insert(&1, &hash);

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
    //     tick: String, // TODO: Change to the time parser type
    //     arguments: String
    // ) -> Task {
    //     // TODO: 
    // }

    /// ```bash
    /// near call cron.testnet remove_task '{"task_hash": ""}' --accountId YOU.testnet
    /// ```
    pub fn remove_task(
        &mut self,
        task_hash: u128,
    ) -> Option<Vec<u8>> {
        // TODO: Add asserts: owner only, 
        self.tabs.remove(&task_hash)
    }

    /// Called directly by a registered agent
    /// ```bash
    /// near call cron.testnet proxy_call --accountId YOU.testnet
    /// ```
    pub fn proxy_call(&mut self) {
        // only registered agent signed, because micropayments will benefit long term
        let mut agent = self.agents.get(&env::signer_account_pk())
            .expect("Agent not registered");

        // TODO: Get current slot based on block or timestamp
        let current_slot = self.current_slot_id();
        log!("current slot {:?}", current_slot);
        let slot = vec![1];

        // get task based on current slot
        let mut task = self.tasks.get(&slot)
            .expect("No tasks found in slot");
        let hash = self.hash(&task);
        let call_balance_used = task.fn_allowance + task.gas_allowance + self.agent_fee;

        assert!(call_balance_used < task.balance, "Not enough task balance to execute job");
            
        // Increment agent rewards
        agent.balance += self.agent_fee;

        // Increment agent task count
        agent.balance += 1;

        // Decrease task balance
        // TODO: Change to real gas used
        task.balance -= call_balance_used;

        // Update storage in both places
        self.agents.insert(&env::signer_account_pk(), &agent);
        self.tasks.insert(&hash, &task);

        // Call external contract with task variables
        env::promise_create(
            task.contract_id,
            &task.function_id.as_bytes(),
            json!({}).to_string().as_bytes(),
            Some(task.fn_allowance).unwrap_or(0),
            env::prepaid_gas()
        );
    }

    /// Keep track of this agent, allows for rewards tracking
    ///
    /// ```bash
    /// near call cron.testnet remove_agent '{"payable_account_id": "YOU.testnet"}' --accountId YOUR_AGENT.testnet
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

    // TODO: Get agent stats

    fn hash(&self, item: &Task) -> Vec<u8> {
        // Generate hash
        let input = format!(
                "{:?}{:?}{:?}",
                item.contract_id,
                item.function_id,
                item.tick
            );
        env::keccak256(input.as_bytes())
    }

    // TODO: this will need a major overhaul, for now simplify!
    /// Returns current slot based on current block height
    /// rounded to nearest granularity (~every 60 blocks)
    fn current_slot_id(&self) -> Vec<u8> {
        let block = env::block_index();
        let rem = block % SLOT_GRANULARITY;
        (block - rem).try_to_vec().unwrap()
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
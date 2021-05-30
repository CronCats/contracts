use near_sdk::{
    AccountId,
    Balance,
    PanicOnDefault,
    Promise,
    PublicKey,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LookupMap, TreeMap},
    env,
    json_types::{ValidAccountId, Base58PublicKey, Base64VecU8, U128},
    log,
    near_bindgen,
    serde::{Deserialize, Serialize},
    serde_json::json,
    Gas,
    PromiseResult,
    base64
};
use cron_schedule::Schedule;
use std::str::FromStr;

near_sdk::setup_alloc!();

// Balance & Fee Definitions
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const GAS_BASE_FEE: Gas = 3_000_000_000_000;
// TODO: investigate how much this should be, currently
// http post https://rpc.mainnet.near.org jsonrpc=2.0 id=dontcare method=EXPERIMENTAL_genesis_config
// > mainnet-config.json

pub const GAS_FOR_CALLBACK: Gas = 75_000_000_000_000;
pub const STAKE_BALANCE_MIN: u128 = 10 * ONE_NEAR;

// Boundary Definitions
pub const MAX_BLOCK_RANGE: u64 = 1_000_000_000_000_000;
pub const MAX_EPOCH_RANGE: u32 = 10_000;
pub const MAX_SECOND_RANGE: u32 = 600_000_000;
pub const SLOT_GRANULARITY: u64 = 60;
pub const NANO: u64 = 1_000_000_000;

/// Allows tasks to be executed in async env
#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum TaskStatus {
    /// Shows a task is not currently active, ready for an agent to take
    Ready,

    /// Shows a task is currently being processed/called
    Active,

    /// Tasks marked as complete are ready for deletion from state. 
    Complete,

    /// Tasks that were known to fail by checking a promise callback.
    Failed
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct Task {
    /// Entity responsible for this task, can change task details
    pub owner_id: AccountId,

    /// Account to direct all execution calls against
    pub contract_id: AccountId,

    /// Contract method this task will be executing
    pub function_id: String,

    /// Crontab Spec String
    /// Defines the interval spacing of execution
    pub cadence: String,

    /// Defines if this task can continue until balance runs out
    pub recurring: bool,

    /// Tasks status forces single executions per interval
    pub status: TaskStatus,

    /// Total balance of NEAR available for current and future executions
    pub total_deposit: Balance,

    /// Configuration of NEAR balance to send to each function call. This is the "amount" for a function call.
    pub deposit: Balance,

    /// Configuration of NEAR balance to attach to each function call. This is the "gas" for a function call.
    pub gas: Gas,

    // NOTE: Only allow static pre-defined bytes
    pub arguments: Vec<u8>
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Agent {
    #[serde(skip)]
    pk: PublicKey,
    #[serde(skip)]
    pub account_id: AccountId,
    pub payable_account_id: AccountId,
    pub balance: Balance,
    pub total_tasks_executed: u128
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CronManager {
    // Runtime
    // TODO: Setup DAO based management & ownership
    // TODO: Add paused to fns
    // TODO: Add fn "tick" for updating bps, staking, etc
    paused: bool,
    owner_id: AccountId,
    owner_pk: PublicKey,
    bps_block: u64,
    bps_timestamp: u64,

    // Basic management
    agents: LookupMap<PublicKey, Agent>,
    slots: TreeMap<u128, Vec<Vec<u8>>>,
    tasks: LookupMap<Vec<u8>, Task>,

    // Economics
    // TODO: Add admin fns to manage these
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
        assert!(!env::state_exists(), "The contract is already initialized");
        CronManager {
            paused: false,
            owner_id: env::signer_account_id(),
            owner_pk: env::signer_account_pk(),
            bps_block: env::block_index(),
            bps_timestamp: env::block_timestamp(),
            tasks: LookupMap::new(b"t"),
            agents: LookupMap::new(b"a"),
            slots: TreeMap::new(b"s"),
            available_balance: 0,
            staked_balance: 0,
            agent_fee: u128::from(GAS_BASE_FEE),
            slot_granularity: SLOT_GRANULARITY
        }
    }

    /// Gets a set of tasks.
    /// Default: Returns the next executable set of tasks hashes.
    /// 
    /// Optional Parameters:
    /// "offset" - An unsigned integer specifying how far in the future to check for tasks that are slotted.
    /// 
    /// ```bash
    /// near view cron.testnet get_tasks
    /// ```
    pub fn get_tasks(&self, offset: Option<u64>) -> Vec<Base64VecU8> {
        let current_slot = self.get_slot_id(offset);

        // Get tasks based on current slot.
        // (Or closest past slot if there are leftovers.)
        let slot_ballpark = self.slots.floor_key(&current_slot);
        if let Some(k) = slot_ballpark {
            let mut ret: Vec<Base64VecU8> = Vec::new();
            let tasks = self.slots.get(&k).unwrap();
            for task in tasks.iter() {
                ret.push(Base64VecU8::from(task.to_vec()));
            }
            ret
        } else {
            vec![Base64VecU8::from(vec![])]
        }
    }

    /// Most useful for debugging at this point.
    pub fn get_all_tasks(&self, slot: Option<U128>) -> Vec<Base64VecU8> {
        let mut ret: Vec<Base64VecU8> = Vec::new();
        if let Some(slot_number) = slot {
            // User specified a slot number, only return tasks in there.
            let tasks_in_slot = self.slots.get(&slot_number.0).expect("Couldn't find tasks for given slot.");
            for task in tasks_in_slot.iter() {
                ret.push(Base64VecU8::from(task.to_vec()));
            }
        } else {
            // Return all slots
            for slot in self.slots.iter() {
                let tasks_in_slot = slot.1;
                for task in tasks_in_slot.iter() {
                    ret.push(Base64VecU8::from(task.to_vec()));
                }
            }
        }
        ret
    }

    /// Gets the data payload of a single task by hash
    ///
    /// ```bash
    /// near view cron.testnet get_task '{"task_hash": "r2Jv…T4U4="}'
    /// ```
    pub fn get_task(&self, task_hash: Base64VecU8) -> Task {
        let task_hash = task_hash.0;
        let task = self.tasks.get(&task_hash)
            .expect("No task found by hash");
        task
    }

    /// Allows any user or contract to pay for future txns based on a specific schedule
    /// contract, function id & other settings. When the task runs out of balance
    /// the task is no longer executed, any additional funds will be returned to task owner.
    ///
    /// ```bash
    /// near call cron.testnet create_task '{"contract_id": "counter.in.testnet","function_id": "increment","cadence": "@daily","recurring": true,"deposit": 0,"gas": 2400000000000}' --accountId YOU.testnet
    /// ```
    #[payable]
    pub fn create_task(
        &mut self,
        contract_id: String,
        function_id: String,
        cadence: String,
        recurring: Option<bool>,
        deposit: Option<Balance>,
        gas: Option<Gas>,
        arguments: Option<Vec<u8>>
    ) -> Base64VecU8 {
        // TODO: Add asserts to check cadence can be parsed
        log!("cadence {}", &cadence.clone());
        let item = Task {
            owner_id: env::signer_account_id(),
            contract_id,
            function_id,
            cadence,
            recurring: recurring.unwrap_or(false),
            status: TaskStatus::Ready,
            total_deposit: env::attached_deposit(),
            deposit: deposit.unwrap_or(0), // for Ⓝ only?
            gas: gas.unwrap_or(GAS_BASE_FEE),
            arguments: arguments.unwrap_or(b"".to_vec())
        };

        // Check that balance is sufficient for 1 execution minimum
        let call_balance_used = self.task_balance_uses(&item);
        assert!(call_balance_used <= item.total_deposit, "Not enough task balance to execute job, need at least {}", call_balance_used);

        let hash = self.hash(&item);
        log!("Task Hash (as bytes) {:?}", &hash);
        // log!("Task data {:?}", &item.to_string());

        // Parse cadence into a future timestamp, then convert to a slot
        let next_slot = self.get_slot_from_cadence(item.cadence.clone());

        // Add task to catalog
        self.tasks.insert(&hash, &item);

        // Get previous task hashes in slot, add as needed
        let default_vec: Vec<Vec<u8>> = Vec::new();
        let mut slot_slots = self.slots.get(&next_slot).unwrap_or(default_vec);
        slot_slots.push(hash.clone());
        log!("Inserting into slot: {}", next_slot);
        self.slots.insert(&next_slot, &slot_slots);

        Base64VecU8::from(hash)
    }

    /// ```bash
    /// near call cron.testnet update_task '{"task_hash": "","cadence": "@weekly","recurring": true,"deposit": 0,"gas": 2400000000000}' --accountId YOU.testnet
    /// ```
    #[payable]
    pub fn update_task(
        &mut self,
        task_hash: Base64VecU8,
        cadence: Option<String>,
        recurring: Option<bool>,
        deposit: Option<u128>,
        gas: Option<Gas>,
        arguments: Option<Vec<u8>>
    ) {
        let task_hash = task_hash.0;
        let mut task = self.tasks.get(&task_hash)
            .expect("No task found by hash");
        
        assert_eq!(task.owner_id, env::predecessor_account_id(), "Only owner can update their task.");

        // Update args that exist
        if cadence.is_some() { task.cadence = cadence.unwrap(); }
        if recurring.is_some() { task.recurring = recurring.unwrap(); }
        if deposit.is_some() { task.deposit = deposit.unwrap(); }
        if gas.is_some() { task.gas = gas.unwrap(); }
        if arguments.is_some() { task.arguments = arguments.unwrap(); }

        // Update task total available balance, if this function was given a deposit.
        if env::attached_deposit() > 0 {
            task.total_deposit += env::attached_deposit();
        }
    }

    /// Deletes a task in its entirety, returning any remaining balance to task owner.
    /// 
    /// ```bash
    /// near call cron.testnet remove_task '{"task_hash": ""}' --accountId YOU.testnet
    /// ```
    pub fn remove_task(&mut self, task_hash: Base64VecU8) {
        let task_hash = task_hash.0;
        let task = self.tasks.get(&task_hash)
            .expect("No task found by hash");

        assert_eq!(task.owner_id, env::predecessor_account_id(), "Only owner can remove their task.");

        // If owner, allow to remove task
        self.exit_task(task_hash);
    }

    /// Internal management of finishing a task.
    /// Responsible for cleaning up storage &
    /// returning any remaining balance to task owner.
    fn exit_task(&mut self, task_hash: Vec<u8>) {
        let task = self.tasks.get(&task_hash)
            .expect("No task found by hash");
        
        // return any balance
        if task.total_deposit > 0 {
            Promise::new(task.owner_id.to_string())
                .transfer(task.total_deposit);
        }

        // remove task
        self.tasks.remove(&task_hash);
    }

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
        // TODO: Adjust this to get slot from treemap floor?
        let current_slot = self.get_slot_id(None);
        log!("current slot {:?}", current_slot);

        // get task based on current slot
        let mut slot = self.slots.get(&current_slot)
            .expect("No tasks found in slot");
        log!("slot {:?}", &slot);

        // TODO: update state, post pop
        // Get a single task hash, then retrieve task details
        let hash = slot.pop().expect("No tasks available");
        let mut task = self.tasks.get(&hash)
            .expect("No task found by hash");
        log!("Found Task {:?}", &task);

        // let hash = self.hash(&task);
        let call_balance_used = self.task_balance_uses(&task);

        assert!(call_balance_used <= task.total_deposit, "Not enough task balance to execute job");

        // Increment agent reward & task count
        agent.balance += self.agent_fee;
        agent.total_tasks_executed += 1;

        // TODO: Process task exit, if no future task can execute

        // only skip scheduling if user didnt intend
        if task.recurring != false {
            let next_slot = self.get_slot_from_cadence(task.cadence.clone());
            assert!(&current_slot < &next_slot, "Cannot execute task in the past");

            // Get previous task hashes in slot, add as needed
            let default_vec: Vec<Vec<u8>> = Vec::new();
            let mut slot_slots = self.slots.get(&next_slot).unwrap_or(default_vec);
            slot_slots.push(hash.clone());
            self.slots.insert(&next_slot, &slot_slots);
        }

        // Update agent storage
        self.agents.insert(&env::signer_account_pk(), &agent);


        // Decrease task balance
        // TODO: Change to real gas used
        task.total_deposit -= self.task_balance_used();

        task.status = TaskStatus::Active;

        // Update task storage
        self.tasks.insert(&hash, &task);

        // Call external contract with task variables
        let promise_first = env::promise_create(
            task.contract_id.clone(),
            &task.function_id.as_bytes(),
            task.arguments.as_slice(),
            // NOTE: Does this work if signer sends NO amount? Who pays??
            task.deposit,
            task.gas
        );
        let promise_second = env::promise_then(
            promise_first,
            env::current_account_id(),
            b"callback_for_proxy_call",
            json!({ "task_hash": hash }).to_string().as_bytes(),
            0,
            GAS_FOR_CALLBACK,
        );
        env::promise_return(promise_second);
    }

    #[private]
    pub fn callback_for_proxy_call(&mut self, task_hash: Vec<u8>) {
        assert_eq!(env::promise_results_count(), 1, "Expected 1 promise result.");
        let mut task = self.tasks.get(&task_hash.clone())
            .expect("No task found by hash");
        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                log!("Task {} completed successfully", base64::encode(task_hash.clone()));
                task.status = TaskStatus::Complete;
            },
            PromiseResult::Failed => {
                task.status = TaskStatus::Failed;
            },
            PromiseResult::NotReady => unreachable!()
        };
        self.tasks.insert(&task_hash, &task);
    }

    /// Add any account as an agent that will be able to execute tasks.
    /// Registering allows for rewards accruing with micro-payments which will accumulate to more long-term.
    /// 
    /// Optional Parameters:
    /// "payable_account_id" - Allows a different account id to be specified, so a user can receive funds at a different account than the agent account.
    ///
    /// ```bash
    /// near call cron.testnet register_agent '{"payable_account_id": "YOU.testnet"}' --accountId YOUR_AGENT.testnet
    /// ```
    #[payable]
    pub fn register_agent(
        &mut self,
        payable_account_id: Option<ValidAccountId>
    ) {
        // TODO: assert that attached deposit is enough to cover storage cost. This is to protect from storage attack.
        // check that account isn't already added
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

    /// Update agent details, specifically the payable account id for an agent.
    ///
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

    /// Removes the agent from the active set of agents.
    /// Withdraws all reward balances to the agent payable account id.
    ///
    /// ```bash
    /// near call cron.testnet unregister_agent --accountId YOUR_AGENT.testnet
    /// ```
    pub fn unregister_agent(&mut self) {
        let pk = env::signer_account_pk();

        // check that signer agent exists
        if let Some(_acct) = self.agents.get(&pk) {
            // Check if there is balance, if any pay rewards to payable account id.
            if _acct.balance > 0 {
                Promise::new(_acct.payable_account_id.to_string())
                    .transfer(_acct.balance);
            }

            // Remove from active agents
            self.agents.remove(&pk);
        } else {
            panic!("No Agent");
        };
    }

    /// Allows an agent to withdraw all rewards, paid to the specified payable account id.
    ///
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
    /// near view cron.testnet get_agent '{"pk": "ed25519:AGENT_PUBLIC_KEY"}'
    /// ```
    pub fn get_agent(&self, pk: Base58PublicKey) -> Agent {
        let agent = self.agents.get(&pk.into())
            .expect("No agent found");

        agent
    }

    fn hash(&self, item: &Task) -> Vec<u8> {
        // Generate hash
        let input = format!(
                "{:?}{:?}{:?}",
                item.contract_id,
                item.function_id,
                item.cadence
            );
        env::keccak256(input.as_bytes())
    }

    /// Returns the base amount required to execute 1 task
    /// Fee breakdown:
    /// - Used Gas: Task Txn Fee Cost
    /// - Agent Fee: Incentivize Execution SLA
    /// 
    /// Task Fee Example:
    /// Gas: 50 Tgas
    /// Agent: 100 Tgas
    /// Total: 150 Tgas
    ///
    /// NOTE: Gas cost includes the cross-contract call & internal logic of this contract.
    /// Direct contract gas fee will be lower than task execution costs.
    fn task_balance_used(&self) -> u128 {
        u128::from(env::used_gas()) + self.agent_fee
    }

    /// Returns the base amount required to execute 1 task
    fn task_balance_uses(&self, task: &Task) -> u128 {
        task.deposit + u128::from(task.gas) + self.agent_fee
    }

    /// If no offset, Returns current slot based on current block height
    /// If offset, Returns next slot based on current block height & integer offset
    /// rounded to nearest granularity (~every 1 block per sec)
    fn get_slot_id(&self, offset: Option<u64>) -> u128 {
        let block = env::block_index();
        let rem = block % self.slot_granularity;
        let block_round = block - rem;
        let id;

        if let Some(o) = offset {
            // NOTE: Assumption here is that the offset will be in seconds. (blocks per second)
            //       Slot granularity will be in minutes (60 blocks per slot)
            let slot_rem = core::cmp::max(o % self.slot_granularity, 1);
            let slot_round = core::cmp::max(o - slot_rem, self.slot_granularity);
            let next = block_round + slot_round;

            if next - block > block_round + MAX_BLOCK_RANGE {
                // Protect against extreme future block schedules
                id = u64::min(next, block_round + MAX_BLOCK_RANGE);
            } else {
                id = next;
            }
        } else {
            id = block_round;
        }

        u128::from(id)
    }

    /// Parse cadence into a schedule
    /// Get next approximate block from a schedule
    /// return slot from the difference of upcoming block and current block
    fn get_slot_from_cadence(&mut self, cadence: String) -> u128 {
        let current_block = env::block_index();
        let current_block_ts = env::block_timestamp();

        // Schedule params
        // NOTE: eventually use TryFrom
        let schedule = Schedule::from_str(&cadence).unwrap();
        let next_ts = schedule.next_after(&current_block_ts).unwrap();
        let next_diff = next_ts - current_block_ts;

        // calculate the average blocks, to get predicted future block
        let blocks_total = core::cmp::max(current_block - self.bps_block, 1);
        let mut bps = (current_block_ts / NANO - self.bps_timestamp / NANO) / blocks_total;

        // Protect against bps being 0
        if bps < 1 { bps = 1; }

        // return upcoming slot
        // Get the next block based on time it takes until next timestamp
        let offset = (next_diff / NANO) / bps;
        let current = self.get_slot_id(None);
        let next_slot = self.get_slot_id(Some(offset));

        if current == next_slot {
            // Add slot granularity to make sure the minimum next slot is a block within next slot granularity range
            current + u128::from(self.slot_granularity)
        } else {
            next_slot
        }
    }

    pub fn update_settings(&mut self, slot_granularity: u64) {
        assert_eq!(self.owner_id, env::signer_account_id(), "Must be owner");
        self.slot_granularity = slot_granularity;
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::json_types::{ValidAccountId};
    use near_sdk::{testing_env, MockedBlockchain};

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id)
            .block_index(1234)
            .block_timestamp(1_600_000_000_000_000_000);
        builder
    }

    #[test]
    fn test_basic_creation() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = CronManager::new();
        testing_env!(context.is_view(true).build());
        assert!(contract.get_all_tasks(None).is_empty());
        testing_env!(context.is_view(false).attached_deposit(3000000000300).build());
        let task_id = contract.create_task(
            "contract.testnet".to_string(),
            "increment".to_string(),
            "@daily".to_string(),
            Some(true),
            Some(100),
            Some(200),
            None,
        );

        testing_env!(context.is_view(true).build());
        assert_eq!(contract.get_all_tasks(None).len(), 1);

        assert_eq!(contract.get_task(task_id), Task {
            owner_id: accounts(1).to_string(), contract_id: "contract.testnet".to_string(), function_id: "increment".to_string(), cadence: "@daily".to_string(), recurring: true, status: TaskStatus::Ready, total_deposit: 3000000000300, deposit: 100, gas: 200, arguments: vec![]
        });
    }

    #[test]
    fn test_get_slot_id_current_block() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = CronManager::new();
        testing_env!(context.is_view(true).build());
        let slot = contract.get_slot_id(None);

        assert_eq!(slot, 1200);
    }

    #[test]
    fn test_get_slot_id_offset_block() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = CronManager::new();
        testing_env!(context.is_view(true).build());
        let slot = contract.get_slot_id(Some(1));

        assert_eq!(slot, 1260);
    }

    #[test]
    fn test_get_slot_id_max_block() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = CronManager::new();
        testing_env!(context.is_view(true).build());
        let slot = contract.get_slot_id(Some(1_000_000_000));

        // ensure even if we pass in a HUGE number, it can only be scheduled UP to the max pre-defined block settings
        assert_eq!(slot, 1_000_001_160);
    }

    #[test]
    fn test_get_slot_id_change_granularity() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = CronManager::new();
        testing_env!(context.is_view(true).build());
        let slot = contract.get_slot_id(None);
        assert_eq!(slot, 1200);

        testing_env!(context.is_view(false).build());
        contract.update_settings(10);
        testing_env!(context.is_view(true).build());
        let slot = contract.get_slot_id(None);
        assert_eq!(slot, 1230);

        testing_env!(context.is_view(false).build());
        contract.update_settings(1);
        testing_env!(context.is_view(true).build());
        let slot = contract.get_slot_id(None);
        assert_eq!(slot, 1234);
    }

    #[test]
    #[should_panic(expected = "Must be owner")]
    fn test_update_settings_fail() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = CronManager::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.slot_granularity, SLOT_GRANULARITY);

        let mut context2 = get_context(accounts(2));
        testing_env!(context2.is_view(false).build());
        contract.update_settings(10);
        testing_env!(context2.is_view(true).build());
        assert_eq!(contract.slot_granularity, 10);
    }

    #[test]
    fn test_update_settings() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = CronManager::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.slot_granularity, SLOT_GRANULARITY);

        testing_env!(context.is_view(false).build());
        contract.update_settings(10);
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.slot_granularity, 10);
    }

    #[test]
    fn test_get_slot_from_cadence() {
        // TODO: These values seem off...
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = CronManager::new();
        testing_env!(context.is_view(true).build());
        let slot1 = contract.get_slot_from_cadence("*/5 * * * * *".to_string()); // Immediately next slot (since every 5 seconds)
        println!("SLOT 1 {}",slot1);
        assert_eq!(slot1, 1260);
        let slot2 = contract.get_slot_from_cadence("* */5 * * * *".to_string()); // Every 5 mins
        println!("SLOT 2 {}",slot2);
        assert_eq!(slot2, 1380);
        let slot3 = contract.get_slot_from_cadence("* * */5 * * *".to_string()); // Every 5th day
        println!("SLOT 3 {}",slot3);
        assert_eq!(slot3, 10380);
        let slot4 = contract.get_slot_from_cadence("* * * */5 * *".to_string()); // Every 5th Month
        println!("SLOT 4 {}",slot4);
        assert_eq!(slot4, 215580);
        let slot5 = contract.get_slot_from_cadence("* * * * */5 *".to_string()); // Every 5th Year
        println!("SLOT 5 {}",slot5);
        assert_eq!(slot5, 4189980);
        // TODO: This is weird/breaking
        let slot6 = contract.get_slot_from_cadence("* * * * * */5".to_string()); // Every ?
        println!("SLOT 6 {}",slot6);
        assert_eq!(slot6, 4189980);
    }
}
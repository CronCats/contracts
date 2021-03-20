use std::convert::TryInto;

use near_sdk::{borsh::{self, BorshDeserialize, BorshSerialize}, env};
use near_sdk::json_types::{ValidAccountId};
use near_sdk::collections::{LookupMap, UnorderedSet};
use near_sdk::{
    log, near_bindgen, setup_alloc, AccountId, Balance, PublicKey, PanicOnDefault, Promise
};

setup_alloc!();

// Balance & Fee Definitions
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const GAS_BASE_FEE: u128 = 3_000_000_000_000;
pub const STAKE_BALANCE_MIN: u128 = 10 * ONE_NEAR;

// Boundary Definitions
pub const MAX_BLOCK_RANGE: u32 = 1_000_000;
pub const MAX_EPOCH_RANGE: u32 = 10_000;
pub const MAX_SECOND_RANGE: u32 = 600_000_000_000;

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
    tasks: LookupMap<Vec<u128>, Task>,
    agents: LookupMap<PublicKey, Agent>,
    tabs: UnorderedSet<Task>,

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
    payable_account_id: ValidAccountId,
    balance: Balance,
    total_tasks_executed: u128
}

#[near_bindgen]
impl CronManager {
    #[init]
    #[payable]
    pub fn new() -> Self {
        CronManager {
            paused: false,
            owner_id: env::signer_account_id(),
            owner_pk: env::signer_account_pk(),
            tasks: LookupMap::new(b"s".to_vec()),
            agents: LookupMap::new(b"a".to_vec()),
            tabs: UnorderedSet::new(b"c".to_vec()),
            available_balance: 0,
            staked_balance: 0,
            agent_fee: GAS_BASE_FEE
        }
    }

    /// Gets next tick immediate tasks. Limited to return only next set of available ex
    // TODO: finish
    pub fn get_tasks(&self) -> UnorderedSet<Task> {
        assert_ne!(self.tabs.len(), 0);
        self.tabs
    }

    #[payable]
    pub fn create_task(
        &mut self,
        contract_id: AccountId,
        function_id: String,
        tick: String, // TODO: Change to the time parser type
        recurring: Option<bool>,
        fn_allowance: Option<Balance>,
        gas_allowance: Option<Balance>
    ) -> Task {
        // TODO: Add asserts, should check that balance can cover 1 task, and storage for a task
        let item = Task {
            owner_id: env::signer_account_id(),
            contract_id,
            function_id,
            tick,
            recurring: Some(recurring).unwrap_or(false),
            status: TaskStatus::Ready,
            balance: env::attached_deposit(),
            fn_allowance: Some(fn_allowance).or(0),
            gas_allowance: Some(gas_allowance).or(GAS_BASE_FEE),
            next_tick: b""
        };

        // Generate hash
        let input = format!(
                "{:?}{:?}{:?}",
                item.contract_id,
                item.function_id,
                item.tick
            );
        let hash = env::keccak256(input.as_bytes());
        let task_hash = String::from_utf8(hash).unwrap_or(0.to_string());
        log!("Task Hash {}", &task_hash);

        // Add tast to catalog
        self.tasks.insert(task_hash, item);

        // TODO: Parse tick, insert in tabs where necessary
        self.tabs.insert(task_hash);

        item
    }

    #[payable]
    pub fn update_task(
        &mut self,
        task_hash: String,
        contract_id: AccountId,
        tick: String, // TODO: Change to the time parser type
        arguments: String
    ) -> Task {
        // TODO: 
    }

    pub fn remove_task(
        &mut self,
        task_hash: String,
    ) -> bool {
        // TODO: Add asserts: owner only, 
        self.tabs.insert(task_hash);
    }

    /// Called directly by a registered agent
    pub fn proxy_call(&mut self) -> Promise {
        // TODO: Add asserts
        // TODO: get task based on current slot
        // TODO: Call external contract with task variables
        // TODO: Increment agent rewards
        // TODO: Increment agent task count
        // TODO: Decrease task balance
    }

    /// Keep track of this agent, allows for rewards tracking
    pub fn register_agent(
        &mut self,
        payable_account_id: ValidAccountId
    ) -> Agent {
        // check that account isnt already added
        if let Some(a) = self.agents.get(&env::signer_account_pk()) {
            panic!("Agent {} already exists", a.account_id);
        };
        let pk = env::signer_account_pk();

        let agent = Agent {
            pk,
            account_id: env::signer_account_id(),
            payable_account_id: payable_account_id | env::signer_account_id(),
            balance: 0,
            total_tasks_executed: 0
        };

        self.agents.insert(pk.try_into(), &agent);

        agent
    }

    pub fn update_agent(
        &mut self,
        payable_account_id: AccountId
    ) {
        let pk = env::signer_account_pk();

        // check that signer agent exists
        if let Some(agent) = self.agents.get(&pk) {
            if (payable_account_id) {
                agent.payable_account_id = payable_account_id;
            }
            self.agents.insert(pk.try_into(), &agent);
        } else {
            panic!("Agent must register");
        };
    }

    pub fn unregister_agent(&mut self) {
        let pk = env::signer_account_pk();

        // check that signer agent exists
        if let Some(acct) = self.agents.get(&pk) {
            self.agents.remove(pk);
        } else {
            panic!("No Agent");
        };
    }

    pub fn withdraw_task_balance(&mut self) -> Promise {
        let agent;
        let pk = env::signer_account_pk();

        // check that signer agent exists
        if Some(agent) = self.agents.get(&pk) {
            assert!(agent.balance > 0, "No Agent balance");
            Promise::new(agent.payable_account_id).transfer(agent.balance)
        } else {
            panic!("No Agent");
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
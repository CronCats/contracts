use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base58PublicKey}; //ValidAccountId
use near_sdk::collections::{LookupMap, LookupSet};
use near_sdk::{
    log, near_bindgen, setup_alloc, AccountId, Balance, PanicOnDefault,
};

setup_alloc!();

// Balance & Fee Definitions
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const AGENT_BASE_FEE: u128 = 1_000_000_000_000_000;
pub const STAKE_BALANCE_MIN: u128 = 10 * ONE_NEAR;

// Boundary Definitions
pub const MAX_BLOCK_RANGE: u32 = 1_000_000;
pub const MAX_EPOCH_RANGE: u32 = 10_000;
pub const MAX_SECOND_RANGE: u32 = 600_000_000_000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CronManager {
    tasks: LookupMap<Vec<u128>, Task>,
    agents: LookupMap<AccountId, Agent>,
    tabs: LookupSet<Tab>,
}

pub struct Task {
    contract_id: AccountId,
    tick: String, // TODO: Change to the time parser type
    allowance: Balance,
    arguments: String // TODO: Test if this is "safe"
}

pub struct Agent {
    pk: Base58PublicKey,
    account_id: AccountId,
    payable_account_id: AccountId,
    balance: Balance,
    ticks: u128
}

pub struct Tab {
    task: Task,
    next_tick: String // TODO: Change to the time parser type
}

#[near_bindgen]
impl CronManager {
    #[init]
    pub fn new() -> Self {
        CronManager {
            tasks: LookupMap::new(b"s".to_vec()),
            agents: LookupMap::new(b"a".to_vec()),
            tabs: LookupSet::new(b"c".to_vec()),
        }
    }

    pub fn get_tasks(&self) -> Task {
    }

    #[payable]
    pub fn create_task(
        &mut self,
        contract_id: AccountId,
        tick: String, // TODO: Change to the time parser type
        total_allowance: Balance,
        arguments: String
    ) -> Task {
    }

    #[payable]
    pub fn update_task(
        &mut self,
        contract_id: AccountId,
        tick: String, // TODO: Change to the time parser type
        arguments: String
    ) -> Task {
    }

    pub fn remove_task(
        &mut self,
        contract_id: AccountId,
        tick: String // TODO: Change to the time parser type
    ) -> bool {
    }

    pub fn proxy_call(
        &mut self,
        task_hash: Vec<u8>,
        payload: String // TODO: Change to serde json blob?
    ) -> Promise {
    }

    pub fn register_agent(
        &mut self,
        payable_account_id: AccountId
    ) -> Agent {
    }

    pub fn update_agent(
        &mut self,
        payable_account_id: AccountId
    ) -> Agent {
    }

    pub fn unregister_agent(&mut self) -> bool {
    }

    pub fn withdraw_task_balance(&mut self, payable_account_id: AccountId) -> {
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
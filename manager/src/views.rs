use crate::*;

#[near_bindgen]
impl Contract {
    /// Returns semver of this contract.
    ///
    /// ```bash
    /// near view cron.in.testnet version
    /// ```
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Gets the configurations and stats
    ///
    /// ```bash
    /// near view cron.testnet get_info
    /// ```
    pub fn get_info(
        &self,
    ) -> (
        // Runtime
        bool,
        AccountId,
        u64,
        u64,
        [u16; 2],
        u128,
        u64,
        u64,
        Balance,
        Balance,
        Balance,
        Balance,
        Gas,
        u64,
        StorageUsage,
    ) {
        (
            self.paused,
            self.owner_id.clone(),
            self.agent_active_queue.len(),
            self.agent_pending_queue.len(),
            self.agent_task_ratio,
            self.agents_eject_threshold,
            self.slots.len(),
            self.tasks.len(),
            self.available_balance,
            self.staked_balance,
            self.agent_fee,
            self.gas_price,
            self.proxy_callback_gas,
            self.slot_granularity,
            self.agent_storage_usage,
        )
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
    pub fn get_tasks(
        &self,
        offset: Option<u64>,
        account_id: Option<ValidAccountId>,
    ) -> (Vec<Base64VecU8>, U128) {
        let current_slot = self.get_slot_id(offset);
        let empty = (vec![], U128::from(current_slot));

        // TODO: IF paused, and agent, return empty (this will cause all agents to pause automatically, to save failed TXN fees)
        // Get tasks only for my agent
        // - Get agent IF account
        // - then check current slot against agent latest executions
        // - if agent has done max slot executions, return empty
        if let Some(id) = account_id {
            if let Some(a) = self.agents.get(&id.to_string()) {
                // Look at previous slot ID
                let last_slot = u128::from(a.slot_execs[0]);
                if current_slot > last_slot + self.agents_eject_threshold {
                    return empty;
                }
            }
        }

        // Get tasks based on current slot.
        // (Or closest past slot if there are leftovers.)
        let slot_ballpark = self.slots.floor_key(&current_slot);
        if let Some(k) = slot_ballpark {
            let ret: Vec<Base64VecU8> = self
                .slots
                .get(&k)
                .unwrap()
                .into_iter()
                .map(Base64VecU8::from)
                .collect();

            (ret, U128::from(current_slot))
        } else {
            empty
        }
    }

    /// Returns task data
    /// Used by the frontend for viewing tasks
    /// REF: https://docs.near.org/docs/concepts/data-storage#gas-consumption-examples-1
    // TODO: Add offset, limit for pagination
    pub fn get_all_tasks(&self, slot: Option<U128>) -> Vec<Task> {
        let mut ret: Vec<Task> = Vec::new();
        if let Some(U128(slot_number)) = slot {
            // User specified a slot number, only return tasks in there.
            let tasks_in_slot = self.slots.get(&slot_number).unwrap_or_default();
            for task_hash in tasks_in_slot.iter() {
                let task = self.tasks.get(&task_hash).expect("No task found by hash");
                ret.push(task);
            }
        } else {
            // Return all tasks
            for (_, task) in self.tasks.iter() {
                ret.push(task);
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
        let task = self.tasks.get(&task_hash).expect("No task found by hash");
        task
    }

    /// Gets amount of tasks alotted for a single agent per slot
    ///
    /// ```bash
    /// near view cron.testnet get_total_tasks_per_agent_per_slot
    /// ```
    pub fn get_total_tasks_per_agent_per_slot(&self) -> u16 {
        // assess if the task ratio would support a new agent
        let [agent_ratio, task_ratio] = self.agent_task_ratio;

        // Math example:
        // ratio [2 agents, 10 tasks]
        // agent can execute 5 tasks per slot
        task_ratio.div_euclid(agent_ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::json_types::ValidAccountId;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    const BLOCK_START_BLOCK: u64 = 52_201_040;
    const BLOCK_START_TS: u64 = 1_624_151_503_447_000_000;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .signer_account_pk(b"ed25519:4ZhGmuKTfQn9ZpHCQVRwEr4JnutL8Uu3kArfxEqksfVM".to_vec())
            .predecessor_account_id(predecessor_account_id)
            .block_index(BLOCK_START_BLOCK)
            .block_timestamp(BLOCK_START_TS);
        builder
    }

    #[test]
    fn test_contract_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new();
        testing_env!(context.is_view(true).build());
        assert!(contract.get_all_tasks(None).is_empty());
    }

    #[test]
    fn test_task_get_only_active() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();

        // Move forward time and blocks to get more accurate bps
        testing_env!(context
            .is_view(false)
            .attached_deposit(1000000000020000000100)
            .block_timestamp(BLOCK_START_TS + (6 * NANO))
            .block_index(BLOCK_START_BLOCK + 6)
            .build());

        // create a some tasks
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "*/10 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "decrement".to_string(),
            "*/10 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        testing_env!(context
            .is_view(false)
            .block_timestamp(BLOCK_START_TS + (120 * NANO))
            .block_index(BLOCK_START_BLOCK + 120)
            .build());
        testing_env!(context.is_view(true).build());
        println!(
            "contract.get_tasks(None) {:?}",
            contract.get_tasks(None, None).0.len()
        );
        assert_eq!(
            contract.get_tasks(None, None).0.len(),
            2,
            "Task amount diff than expected"
        );

        // change the tasks status
        // contract.proxy_call();
        // testing_env!(context.is_view(true).build());
        // assert_eq!(contract.get_tasks(Some(2)).0.len(), 0, "Task amount should be less");
    }
}

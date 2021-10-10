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
        U64,
        U64,
        [u64; 2],
        U128,
        U64,
        U64,
        U128,
        U128,
        U128,
        U128,
        U64,
        U64,
        U64,
    ) {
        (
            self.paused,
            self.owner_id.clone(),
            U64::from(self.agent_active_queue.len()),
            U64::from(self.agent_pending_queue.len()),
            self.agent_task_ratio,
            U128::from(self.agents_eject_threshold),
            U64::from(self.slots.len()),
            U64::from(self.tasks.len()),
            U128::from(self.available_balance),
            U128::from(self.staked_balance),
            U128::from(self.agent_fee),
            U128::from(self.gas_price),
            U64::from(self.proxy_callback_gas),
            U64::from(self.slot_granularity),
            U64::from(self.agent_storage_usage),
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

        // IF paused, and agent, return empty (this will cause all agents to pause automatically, to save failed TXN fees)
        // Get tasks only for my agent
        if !self.paused {
            if let Some(id) = account_id {
                if let Some(a) = self.agents.get(&id.to_string()) {
                    // Return nothing if agent has missed total threshold
                    let last_slot = a.last_missed_slot;
                    if current_slot
                        > last_slot
                            + (self.agents_eject_threshold * u128::from(self.slot_granularity))
                    {
                        return empty;
                    }

                    // Get slot total to test agent in slot
                    // get task based on current slot, priority goes to tasks that have fallen behind (using floor key)
                    let slot_opt = if let Some(k) = self.slots.floor_key(&current_slot) {
                        self.slots.get(&k)
                    } else {
                        self.slots.get(&current_slot)
                    };
                    let slot_data = slot_opt.unwrap_or_default();

                    // Otherwise, assess if they are in active set, or are able to cover an agent that missed previous slot
                    let (can_execute, _) =
                        self.check_agent_can_execute(id.to_string(), slot_data.len() as u64);
                    if !can_execute {
                        return empty;
                    }
                }
            }
        } else {
            return empty;
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
    pub fn get_all_tasks(
        &self,
        slot: Option<U128>,
        from_index: Option<U64>,
        limit: Option<U64>,
    ) -> Vec<Task> {
        let mut ret: Vec<Task> = Vec::new();
        if let Some(U128(slot_number)) = slot {
            // User specified a slot number, only return tasks in there.
            let tasks_in_slot = self.slots.get(&slot_number).unwrap_or_default();
            for task_hash in tasks_in_slot.iter() {
                let task = self.tasks.get(&task_hash).expect("No task found by hash");
                ret.push(task);
            }
        } else {
            let mut start = 0;
            let mut end = 10;
            if let Some(from_index) = from_index {
                start = from_index.0;
            }
            if let Some(limit) = limit {
                end = u64::min(start + limit.0, self.tasks.len());
            }

            // Return all tasks within range
            let keys = self.tasks.keys_as_vector();
            for i in start..end {
                if let Some(task_hash) = keys.get(i) {
                    if let Some(task) = self.tasks.get(&task_hash) {
                        ret.push(task);
                    }
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
        let task = self.tasks.get(&task_hash).expect("No task found by hash");
        task
    }

    /// Check if agent is able to execute a task
    /// Returns bool and the agents index
    ///
    /// ```bash
    /// near view cron.testnet check_agent_can_execute
    /// ```
    pub fn check_agent_can_execute(
        &self,
        account_id: AccountId,
        slot_tasks_remaining: u64,
    ) -> (bool, u64) {
        // get the index this agent
        let index = self
            .agent_active_queue
            .iter()
            .position(|x| x == account_id)
            .unwrap_or_else(|| 0) as u64;
        let active_index = self.agent_active_index as u64;

        // check if agent index is within range of current index and slot tasks remaining
        (
            index == active_index
                || (index > active_index && index <= active_index + slot_tasks_remaining),
            index,
        )
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
        assert!(contract.get_all_tasks(None, None, None).is_empty());
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

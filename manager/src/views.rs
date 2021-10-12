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
    /// requires other logic to satisfy that there is a task to do, outside this function
    ///
    /// ```bash
    /// near view cron.testnet check_agent_can_execute '{"account_id": "YOU.testnet", "slot_tasks_remaining": 3}'
    /// ```
    pub fn check_agent_can_execute(
        &self,
        account_id: AccountId,
        slot_tasks_remaining: u64,
    ) -> (bool, u64) {
        // get the index this agent
        let index_raw = self
            .agent_active_queue
            .iter()
            .position(|x| x == account_id);
        let active_index = self.agent_active_index as u64;
        let agents_total = self.agent_active_queue.len();
        let mut index: u64 = 0;
        log!("agent_active_queue {:?} {:?}", self.agent_active_queue.get(0), self.agent_active_queue.get(1));
        log!("agent_pending_queue {:?} {:?}", self.agent_pending_queue.get(0), self.agent_pending_queue.get(1));
        log!("index_raw active_index {:?} {:?}", index_raw, active_index);

        if let Some(index_raw) = index_raw {
            index = index_raw as u64;
            log!("HERERE {:?}", index);
        } else {
            log!("HERERfdjkfjdkfdjlE");
            return (false, index)
        }
        log!("tally tasks, agents {:?} {:?}", slot_tasks_remaining, agents_total);

        // return immediately if no tasks LOL
        if slot_tasks_remaining == 0 { return (false, index) }

        // check if agent index is within range of current index and slot tasks remaining
        // Single Agent: Return Always
        if agents_total <= 1 {
            log!("single agent {:?}", account_id);
            return (true, index)
        }

        // If 1 task remaining in this slot, only active_index agent
        if slot_tasks_remaining <= 1 {
            log!("single task {:?} {:?}", slot_tasks_remaining, account_id);
            return (index == active_index, index)
        }
        log!("many task {:?} {:?} {:?} {:?} {:?}", slot_tasks_remaining, index, active_index, index == active_index, account_id);

        // Plethora of tasks:
        if slot_tasks_remaining > agents_total {
            return (true, index)
        }

        // // For multiple tasks, get the upper bound index to test against, since we already know active index.
        // // NOTE: needs to accomodate the wrap around scenarios (2 agents, 5 tasks)
        // let upper_bound = active_index + slot_tasks_remaining;
        // let tasks_rem = slot_tasks_remaining % agents_total;
        // let tasks_rounded = slot_tasks_remaining.saturating_sub(tasks_rem);

        // align the amount of agents and available tasks
        // TODO: Handle the wrap around case!
        (index >= active_index && index <= active_index + slot_tasks_remaining, index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::json_types::ValidAccountId;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    const BLOCK_START_TS: u64 = 1_624_151_503_447_000_000;
    const AGENT_STORAGE_FEE: u128 = 2260000000000000000000;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .signer_account_pk(b"ed25519:4ZhGmuKTfQn9ZpHCQVRwEr4JnutL8Uu3kArfxEqksfVM".to_vec())
            .predecessor_account_id(predecessor_account_id)
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
    }

    // 1 agent, always
    #[test]
    fn test_check_agent_can_execute_single_agent() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();

        // Move forward time and blocks to get more accurate bps
        testing_env!(context
            .is_view(false)
            .attached_deposit(1000000000020000000100)
            .block_timestamp(BLOCK_START_TS)
            .build());

        // create a some tasks
        contract.create_task(accounts(3), "increment".to_string(), "*/1 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        contract.create_task(accounts(3), "decrement".to_string(), "*/2 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        
        // Register an agent
        testing_env!(context.is_view(false).attached_deposit(AGENT_STORAGE_FEE).predecessor_account_id(accounts(4)).build());
        contract.register_agent(Some(accounts(4)));
        testing_env!(context.is_view(false).block_timestamp(BLOCK_START_TS + (120 * NANO)).predecessor_account_id(accounts(4)).build());
        let (can_exec, index) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(
            can_exec,
            true,
            "Can execute: Single Agent: True"
        );
        assert_eq!(
            index,
            0,
            "Can execute: Single Agent: Index 0"
        );
        testing_env!(context.is_view(false).block_timestamp(BLOCK_START_TS + (240 * NANO)).predecessor_account_id(accounts(4)).build());
        let (can_exec_2, index_2) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(
            can_exec_2,
            true,
            "Can execute: Single Agent: True"
        );
        assert_eq!(
            index_2,
            0,
            "Can execute: Single Agent: Index 0"
        );
    }

    #[test]
    fn test_check_agent_can_execute_multi_agent_one_task() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();

        // Move forward time and blocks to get more accurate bps
        testing_env!(context
            .is_view(false)
            .attached_deposit(1000000000020000000100)
            .block_timestamp(BLOCK_START_TS)
            .build());

        // create a some tasks
        contract.create_task(accounts(3), "increment".to_string(), "*/1 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        contract.create_task(accounts(3), "increment".to_string(), "*/2 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        contract.create_task(accounts(3), "increment".to_string(), "*/3 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        contract.create_task(accounts(3), "increment".to_string(), "*/4 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        testing_env!(context.is_view(false).block_timestamp(BLOCK_START_TS + (120 * NANO)).build());
        
        // Register an agent
        testing_env!(context.is_view(false).attached_deposit(AGENT_STORAGE_FEE).predecessor_account_id(accounts(4)).build());
        contract.register_agent(Some(accounts(4)));
        testing_env!(context.is_view(false).attached_deposit(AGENT_STORAGE_FEE).predecessor_account_id(accounts(5)).build());
        contract.register_agent(Some(accounts(5)));
        contract.tick();
        testing_env!(context.is_view(true).build());
        let (can_exec, index) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec, true, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        let (can_exec_2, index_2) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
        assert_eq!(can_exec_2, false, "Can execute: Multi Agent: False");
        assert_eq!(index_2, 1, "Can execute: Multi Agent: Index 0");

        // active index shift
        testing_env!(context.is_view(false).block_timestamp(BLOCK_START_TS + (240 * NANO)).build());
        contract.agent_active_index = 1;
        testing_env!(context.is_view(true).build());
        let (can_exec, index) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec, false, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        let (can_exec_2, index_2) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
        assert_eq!(can_exec_2, true, "Can execute: Multi Agent: False");
        assert_eq!(index_2, 1, "Can execute: Multi Agent: Index 0");
    }

    // 2 agents, 1 at a time (more agents than tasks in this slot)
    //   - 2 agents, 1 task
    //   - 3 agents, 2 tasks
    //   - 4 agents, 3 tasks
    #[test]
    fn test_check_agent_can_execute_multi_agent_gt_multi_task() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();

        // Move forward time and blocks to get more accurate bps
        testing_env!(context
            .is_view(false)
            .attached_deposit(1000000000020000000100)
            .block_timestamp(BLOCK_START_TS)
            .build());

        // create a some tasks
        contract.create_task(accounts(3), "increment".to_string(), "*/1 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        contract.create_task(accounts(3), "decrement".to_string(), "*/2 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        contract.create_task(accounts(3), "increment".to_string(), "*/3 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        contract.create_task(accounts(3), "decrement".to_string(), "*/4 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        testing_env!(context.is_view(false).block_timestamp(BLOCK_START_TS + (120 * NANO)).build());
        
        // Register an agent
        testing_env!(context.is_view(false).attached_deposit(AGENT_STORAGE_FEE).predecessor_account_id(accounts(4)).build());
        contract.register_agent(Some(accounts(4)));
        testing_env!(context.is_view(false).attached_deposit(AGENT_STORAGE_FEE).predecessor_account_id(accounts(5)).build());
        contract.register_agent(Some(accounts(5)));
        contract.tick();
        // testing_env!(context.is_view(true).build());
        let (can_exec, index) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec, true, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        contract.agent_active_index = 1;
        let (can_exec_2, index_2) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
        assert_eq!(can_exec_2, true, "Can execute: Multi Agent: True");
        assert_eq!(index_2, 1, "Can execute: Multi Agent: Index 0");

        // active index shift
        testing_env!(context.is_view(false).block_timestamp(BLOCK_START_TS + (240 * NANO)).build());
        contract.agent_active_index = 0;
        // testing_env!(context.is_view(true).build());
        let (can_exec, index) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec, true, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        contract.agent_active_index = 1;
        let (can_exec_2, index_2) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
        assert_eq!(can_exec_2, true, "Can execute: Multi Agent: True");
        assert_eq!(index_2, 1, "Can execute: Multi Agent: Index 0");
    }

    // 2 agents, 3 tasks per slot (more tasks than agents in this slot)
    //   - 2 agents, 3 tasks
    //   - 3 agents, 4 tasks
    //   - 4 agents, 5 tasks
    #[test]
    fn test_check_agent_can_execute_multi_agent_lt_multi_task() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();

        // Move forward time and blocks to get more accurate bps
        testing_env!(context
            .is_view(false)
            .attached_deposit(1000000000020000000100)
            .block_timestamp(BLOCK_START_TS)
            .build());

        // create a some tasks
        contract.create_task(accounts(3), "increment".to_string(), "*/1 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        contract.create_task(accounts(3), "decrement".to_string(), "*/1 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None);
        contract.create_task(accounts(3), "excrement".to_string(), "*/1 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None); // #poojokes
        contract.create_task(accounts(3), "excitement".to_string(), "*/1 * * * * *".to_string(), Some(false), Some(U128::from(0)), Some(200), None); // #poojokes
        testing_env!(context.is_view(false).block_timestamp(BLOCK_START_TS + (120 * NANO)).build());
        
        // Register an agent
        testing_env!(context.is_view(false).attached_deposit(AGENT_STORAGE_FEE).predecessor_account_id(accounts(4)).build());
        contract.register_agent(Some(accounts(4)));
        testing_env!(context.is_view(false).attached_deposit(AGENT_STORAGE_FEE).predecessor_account_id(accounts(5)).build());
        contract.register_agent(Some(accounts(5)));
        contract.tick();
        // testing_env!(context.is_view(true).build());
        let (can_exec, index) = contract.check_agent_can_execute(accounts(4).to_string(), 3);
        assert_eq!(can_exec, true, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        contract.agent_active_index = 1;
        let (can_exec_2, index_2) = contract.check_agent_can_execute(accounts(5).to_string(), 2);
        assert_eq!(can_exec_2, true, "Can execute: Multi Agent: True");
        assert_eq!(index_2, 1, "Can execute: Multi Agent: Index 1");
        contract.agent_active_index = 0;
        let (can_exec_3, index_3) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec_3, true, "Can execute: Multi Agent: True");
        assert_eq!(index_3, 0, "Can execute: Multi Agent: Index 0");
        let (can_exec_4, index_4) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
        assert_eq!(can_exec_4, false, "Can execute: Multi Agent: False");
        assert_eq!(index_4, 1, "Can execute: Multi Agent: Index 1");
        contract.agent_active_index = 1;
        let (can_exec_5, index_5) = contract.check_agent_can_execute(accounts(4).to_string(), 0);
        assert_eq!(can_exec_5, false, "Can execute: Multi Agent: True");
        assert_eq!(index_5, 0, "Can execute: Multi Agent: Index 0");
        let (can_exec_6, index_6) = contract.check_agent_can_execute(accounts(5).to_string(), 0);
        assert_eq!(can_exec_6, false, "Can execute: Multi Agent: False");
        assert_eq!(index_6, 1, "Can execute: Multi Agent: Index 1");
    }
}

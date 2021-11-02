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
        U128,
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
            U128::from(env::account_balance()),
        )
    }

    /// Gets a set of tasks.
    /// Default: Returns the next executable set of tasks hashes.
    ///
    /// Optional Parameters:
    /// "offset" - An unsigned integer specifying how far in the future to check for tasks that are slotted.
    ///
    /// ```bash
    /// near view cron.testnet get_slot_tasks
    /// ```
    pub fn get_slot_tasks(&self, offset: Option<u64>) -> (Vec<Base64VecU8>, U128) {
        let current_slot = self.get_slot_id(offset);
        let empty = (vec![], U128::from(current_slot));

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

    /// Gets list of active slot ids
    ///
    /// ```bash
    /// near view cron.testnet get_slot_ids
    /// ```
    pub fn get_slot_ids(&self) -> Vec<U128> {
        self.slots
            .to_vec()
            .iter()
            .map(|i| U128::from(i.0))
            .collect()
    }

    /// Returns task data
    /// Used by the frontend for viewing tasks
    /// REF: https://docs.near.org/docs/concepts/data-storage#gas-consumption-examples-1
    ///
    /// ```bash
    /// near view cron.testnet get_tasks '{"from_index": 0, "limit": 10}'
    /// ```
    pub fn get_tasks(
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

    /// Get the hash of a task based on parameters
    ///
    /// ```bash
    /// near view cron.testnet get_hash '{"contract_id": "YOUR_CONTRACT.near","function_id": "METHOD_NAME","cadence": "0 0 */1 * * *","owner_id": "YOUR_ACCOUNT.near"}'
    /// ```
    pub fn get_hash(
        &self,
        contract_id: String,
        function_id: String,
        cadence: String,
        owner_id: AccountId,
    ) -> Base64VecU8 {
        // Generate hash, needs to be from known values so we can reproduce the hash without storing
        let input = format!(
            "{:?}{:?}{:?}{:?}",
            contract_id, function_id, cadence, owner_id
        );
        Base64VecU8::from(env::sha256(input.as_bytes()))
    }

    /// Gets list of agent ids
    ///
    /// ```bash
    /// near view cron.testnet get_agent_ids
    /// ```
    pub fn get_agent_ids(&self) -> (String, String) {
        let comma: &str = ",";
        (
            self.agent_active_queue.iter().map(|a| a + comma).collect(),
            // self.agent_active_queue.iter().collect(),
            self.agent_pending_queue.iter().map(|a| a + comma).collect(),
        )
    }

    /// Check how many tasks an agent can execute
    ///
    /// ```bash
    /// near view cron.testnet get_agent_tasks '{"account_id": "YOUR_AGENT.testnet"}'
    /// ```
    pub fn get_agent_tasks(&self, account_id: ValidAccountId) -> (U64, U128) {
        let current_slot = self.get_slot_id(None);
        let empty = (U64::from(0), U128::from(current_slot));

        // IF paused, return empty (this will cause all agents to pause automatically, to save failed TXN fees)
        // Paused will show as timestamp 0 to agent
        if self.paused {
            return (U64::from(0), U128::from(0));
        }

        // Get tasks only for THIS agent
        if let Some(a) = self.agents.get(&account_id.to_string()) {
            // Return nothing if agent has missed total threshold
            let last_slot = a.last_missed_slot;
            if a.last_missed_slot != 0
                && (current_slot
                    > last_slot + (self.agents_eject_threshold * u128::from(self.slot_granularity)))
            {
                return empty;
            }

            // Skip if agent is not active
            if a.status != agent::AgentStatus::Active {
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
            let (can_execute, _, agent_tasks) =
                self.check_agent_can_execute(account_id.to_string(), slot_data.len() as u64);
            if !can_execute {
                return empty;
            }

            // Available tasks to only THIS agent!
            // NOTE: Don't need to know any hashes, just how many tasks.
            return (U64::from(agent_tasks), U128::from(current_slot));
        }

        empty
    }

    /// Check if agent is able to execute a task
    /// Returns bool and the agents index
    /// requires other logic to satisfy that there is a task to do, outside this function
    ///
    /// Response: (canExecute: bool, agentIndex: u64, tasksAvailable: u64)
    ///
    /// ```bash
    /// near view cron.testnet check_agent_can_execute '{"account_id": "YOU.testnet", "slot_tasks_remaining": 3}'
    /// ```
    // NOTE: How does async affect this?
    pub fn check_agent_can_execute(
        &self,
        account_id: AccountId,
        slot_tasks_remaining: u64,
    ) -> (bool, u64, u64) {
        // get the index this agent
        let index_raw = self.agent_active_queue.iter().position(|x| x == account_id);
        let active_index = self.agent_active_index as u64;
        let agents_total = self.agent_active_queue.len();
        let mut index: u64 = 0;

        if let Some(index_raw) = index_raw {
            index = index_raw as u64;
        } else {
            return (false, index, 0);
        }

        // return immediately if no tasks LOL
        if slot_tasks_remaining == 0 {
            return (false, index, 0);
        }

        // check if agent index is within range of current index and slot tasks remaining
        // Single Agent: Return Always
        if agents_total <= 1 {
            return (true, index, slot_tasks_remaining);
        }

        // If 1 task remaining in this slot, only active_index agent
        // NOTE: This is possibly affected by async misfire?
        if slot_tasks_remaining <= 1 {
            return (
                index == active_index,
                index,
                u64::max(slot_tasks_remaining, 1),
            );
        }

        // Plethora of tasks:
        //
        // Examples:
        // agent ids: [0,1,2,3,4,5] :: Tasks 7 :: Active Index 0 :: Active Agents [0,1,2,3,4,5]
        // agent ids: [0,1,2,3,4,5] :: Tasks 3 :: Active Index 0 :: Active Agents [0,1,2]
        if slot_tasks_remaining > agents_total {
            return (
                true,
                index,
                // TODO: Change to make sure theres not an accidental 1
                u64::max(slot_tasks_remaining.div_euclid(agents_total), 1),
            );
        }

        // Align the amount of agents and available tasks
        // Easiest method is to split the range in two and compare
        //
        // Example:
        // agent ids: [0,1,2,3,4,5] :: Tasks 3 :: Active Index 4 :: Active Agents [4,5,0]
        let total_agents = self.agent_active_queue.len().saturating_sub(1);
        let right_upper_bound = u64::min(active_index + slot_tasks_remaining, total_agents);
        let left_upper_bound = (active_index + slot_tasks_remaining) - total_agents;

        // Compare right boundary
        // agent ids: [0,1,2,3,4,5] :: Tasks 3 :: Active Index 4 :: Agent 5 :: Active Agents [4,5,0]
        // TODO: Create test for this case
        if active_index <= index && index <= right_upper_bound {
            return (true, index, 1);
        }

        // Compare left boundary
        // agent ids: [0,1,2,3,4,5] :: Tasks 3 :: Active Index 4 :: Agent 0 :: Active Agents [4,5,0]
        // TODO: Create test for this case
        (active_index <= index && index <= left_upper_bound, index, 1)
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
        assert!(contract.get_tasks(None, None, None).is_empty());
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
            contract.get_slot_tasks(None).0.len()
        );
        assert_eq!(
            contract.get_slot_tasks(None).0.len(),
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
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "*/1 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "decrement".to_string(),
            "*/2 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );

        // Register an agent
        testing_env!(context
            .is_view(false)
            .attached_deposit(AGENT_STORAGE_FEE)
            .predecessor_account_id(accounts(4))
            .build());
        contract.register_agent(Some(accounts(4)));
        testing_env!(context
            .is_view(false)
            .block_timestamp(BLOCK_START_TS + (120 * NANO))
            .predecessor_account_id(accounts(4))
            .build());
        let (can_exec, index, _) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec, true, "Can execute: Single Agent: True");
        assert_eq!(index, 0, "Can execute: Single Agent: Index 0");
        testing_env!(context
            .is_view(false)
            .block_timestamp(BLOCK_START_TS + (240 * NANO))
            .predecessor_account_id(accounts(4))
            .build());
        let (can_exec_2, index_2, _) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec_2, true, "Can execute: Single Agent: True");
        assert_eq!(index_2, 0, "Can execute: Single Agent: Index 0");
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
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "*/1 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "*/2 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "*/3 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "*/4 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        testing_env!(context
            .is_view(false)
            .block_timestamp(BLOCK_START_TS + (120 * NANO))
            .build());

        // Register an agent
        testing_env!(context
            .is_view(false)
            .attached_deposit(AGENT_STORAGE_FEE)
            .predecessor_account_id(accounts(4))
            .build());
        contract.register_agent(Some(accounts(4)));
        testing_env!(context
            .is_view(false)
            .attached_deposit(AGENT_STORAGE_FEE)
            .predecessor_account_id(accounts(5))
            .build());
        contract.register_agent(Some(accounts(5)));
        contract.tick();
        testing_env!(context.is_view(true).build());
        let (can_exec, index, _) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec, true, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        let (can_exec_2, index_2, _) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
        assert_eq!(can_exec_2, false, "Can execute: Multi Agent: False");
        assert_eq!(index_2, 1, "Can execute: Multi Agent: Index 0");

        // active index shift
        testing_env!(context
            .is_view(false)
            .block_timestamp(BLOCK_START_TS + (240 * NANO))
            .build());
        contract.agent_active_index = 1;
        testing_env!(context.is_view(true).build());
        let (can_exec, index, _) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec, false, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        let (can_exec_2, index_2, _) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
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
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "*/1 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "decrement".to_string(),
            "*/2 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "*/3 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "decrement".to_string(),
            "*/4 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        testing_env!(context
            .is_view(false)
            .block_timestamp(BLOCK_START_TS + (120 * NANO))
            .build());

        // Register an agent
        testing_env!(context
            .is_view(false)
            .attached_deposit(AGENT_STORAGE_FEE)
            .predecessor_account_id(accounts(4))
            .build());
        contract.register_agent(Some(accounts(4)));
        testing_env!(context
            .is_view(false)
            .attached_deposit(AGENT_STORAGE_FEE)
            .predecessor_account_id(accounts(5))
            .build());
        contract.register_agent(Some(accounts(5)));
        contract.tick();
        // testing_env!(context.is_view(true).build());
        let (can_exec, index, _) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec, true, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        contract.agent_active_index = 1;
        let (can_exec_2, index_2, _) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
        assert_eq!(can_exec_2, true, "Can execute: Multi Agent: True");
        assert_eq!(index_2, 1, "Can execute: Multi Agent: Index 0");

        // active index shift
        testing_env!(context
            .is_view(false)
            .block_timestamp(BLOCK_START_TS + (240 * NANO))
            .build());
        contract.agent_active_index = 0;
        // testing_env!(context.is_view(true).build());
        let (can_exec, index, _) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec, true, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        contract.agent_active_index = 1;
        let (can_exec_2, index_2, _) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
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
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "*/1 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "decrement".to_string(),
            "*/1 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        );
        contract.create_task(
            accounts(3),
            "excrement".to_string(),
            "*/1 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        ); // #poojokes
        contract.create_task(
            accounts(3),
            "excitement".to_string(),
            "*/1 * * * * *".to_string(),
            Some(false),
            Some(U128::from(0)),
            Some(200),
            None,
        ); // #poojokes
        testing_env!(context
            .is_view(false)
            .block_timestamp(BLOCK_START_TS + (120 * NANO))
            .build());

        // Register an agent
        testing_env!(context
            .is_view(false)
            .attached_deposit(AGENT_STORAGE_FEE)
            .predecessor_account_id(accounts(4))
            .build());
        contract.register_agent(Some(accounts(4)));
        testing_env!(context
            .is_view(false)
            .attached_deposit(AGENT_STORAGE_FEE)
            .predecessor_account_id(accounts(5))
            .build());
        contract.register_agent(Some(accounts(5)));
        contract.tick();
        // testing_env!(context.is_view(true).build());
        let (can_exec, index, _) = contract.check_agent_can_execute(accounts(4).to_string(), 3);
        assert_eq!(can_exec, true, "Can execute: Multi Agent: True");
        assert_eq!(index, 0, "Can execute: Multi Agent: Index 0");
        contract.agent_active_index = 1;
        let (can_exec_2, index_2, _) = contract.check_agent_can_execute(accounts(5).to_string(), 2);
        assert_eq!(can_exec_2, true, "Can execute: Multi Agent: True");
        assert_eq!(index_2, 1, "Can execute: Multi Agent: Index 1");
        contract.agent_active_index = 0;
        let (can_exec_3, index_3, _) = contract.check_agent_can_execute(accounts(4).to_string(), 1);
        assert_eq!(can_exec_3, true, "Can execute: Multi Agent: True");
        assert_eq!(index_3, 0, "Can execute: Multi Agent: Index 0");
        let (can_exec_4, index_4, _) = contract.check_agent_can_execute(accounts(5).to_string(), 1);
        assert_eq!(can_exec_4, false, "Can execute: Multi Agent: False");
        assert_eq!(index_4, 1, "Can execute: Multi Agent: Index 1");
        contract.agent_active_index = 1;
        let (can_exec_5, index_5, _) = contract.check_agent_can_execute(accounts(4).to_string(), 0);
        assert_eq!(can_exec_5, false, "Can execute: Multi Agent: True");
        assert_eq!(index_5, 0, "Can execute: Multi Agent: Index 0");
        let (can_exec_6, index_6, _) = contract.check_agent_can_execute(accounts(5).to_string(), 0);
        assert_eq!(can_exec_6, false, "Can execute: Multi Agent: False");
        assert_eq!(index_6, 1, "Can execute: Multi Agent: Index 1");
    }
}

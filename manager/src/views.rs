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

    /// Gets a set of tasks.
    /// Default: Returns the next executable set of tasks hashes.
    ///
    /// Optional Parameters:
    /// "offset" - An unsigned integer specifying how far in the future to check for tasks that are slotted.
    ///
    /// ```bash
    /// near view cron.testnet get_tasks
    /// ```
    pub fn get_tasks(&self, account_id: Option<ValidAccountId>, offset: Option<u64>) -> (Vec<Base64VecU8>, U128) {
        let current_slot = self.get_slot_id(offset);

        // // TODO: Get tasks only for my agent
        // // Get agent IF account, then check current slot and if agent has done X executions
        // if let Some(id) = account_id {
        //     if let Some(a) = self.agents.get(id) {
        //         // Look at previous slot ID

        //     }
        // }

        // Get tasks based on current slot.
        // (Or closest past slot if there are leftovers.)
        let slot_ballpark = self.slots.floor_key(&current_slot);
        if let Some(k) = slot_ballpark {
            let ret: Vec<Base64VecU8> =
                self.slots.get(&k)
                    .unwrap()
                    .into_iter()
                    .map(Base64VecU8::from)
                    .collect();

            (ret, U128::from(current_slot))
        } else {
            (vec![], U128::from(current_slot))
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
            let tasks_in_slot = self
                .slots
                .get(&slot_number)
                .unwrap_or_default();
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
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::json_types::ValidAccountId;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    use chrono::prelude::DateTime;
    use chrono::Utc;
    use chrono::*;

    const BLOCK_START_BLOCK: u64 = 52_201_040;
    const BLOCK_START_TS: u64 = 1_624_151_503_447_000_000;

    pub fn get_sample_task() -> Task {
        Task {
            owner_id: String::from("bob"),
            contract_id: String::from("danny"),
            function_id: String::from("increment"),
            cadence: String::from("0 0 */1 * * *"),
            recurring: false,
            total_deposit: U128::from(1000000000020000000100),
            deposit: U128::from(100),
            gas: 200,
            arguments: Base64VecU8::from(vec![]),
        }
    }

    // from https://stackoverflow.com/a/50072164/711863
    pub fn human_readable_time(time_nano: u64) -> String {
        let timestamp = (time_nano / 1_000_000_000)
            .to_string()
            .parse::<i64>()
            .unwrap();
        let naive = NaiveDateTime::from_timestamp(timestamp, 0);
        let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
        let newdate = datetime.format("%Y-%m-%d %H:%M:%S");
        // Print the newly formatted date and time
        newdate.to_string()
    }

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
            .attached_deposit(3000000000000300)
            .block_timestamp(BLOCK_START_TS + (12 * NANO))
            .block_index(BLOCK_START_BLOCK + 12)
            .build());
        testing_env!(context.is_view(true).build());
        println!(
            "contract.get_tasks(None) {:?}",
            contract.get_tasks(Some(accounts(1)), None).0.len()
        );
        assert_eq!(
            contract.get_tasks(Some(accounts(1)), None).0.len(),
            2,
            "Task amount diff than expected"
        );

        // change the tasks status
        // contract.proxy_call();
        // testing_env!(context.is_view(true).build());
        // assert_eq!(contract.get_tasks(Some(2)).0.len(), 0, "Task amount should be less");
    }
}
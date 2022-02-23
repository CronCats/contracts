use crate::*;
use near_sdk::serde_json;

pub const NO_DEPOSIT: Balance = 0;
pub const VIEW_CALL_GAS: Gas = 240_000_000_000_000;

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct Trigger {
    /// Entity responsible for this task, can change task details
    pub owner_id: AccountId,

    /// Account to direct all view calls against
    pub contract_id: AccountId,

    /// Contract method this trigger will be viewing
    pub function_id: String,

    // NOTE: Only allow static pre-defined bytes
    pub arguments: Base64VecU8,

    /// The task to trigger if view results in TRUE
    /// Task can still use a cadence, or can utilize a very large time window and allow view triggers to be main source of execution
    pub task_hash: Base64VecU8,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct TriggerHumanFriendly {
    pub owner_id: AccountId,
    pub contract_id: AccountId,
    pub function_id: String,
    pub arguments: Base64VecU8,
    pub task_hash: Base64VecU8,
    pub hash: Base64VecU8,
}

pub type CroncatTriggerResponse = (bool, Option<Base64VecU8>);

#[near_bindgen]
impl Contract {
    /// !IMPORTANT!:: BETA FEATURE!!!!!!!!!
    /// Configure a VIEW call to map to a task, allowing IFTTT functionality
    /// IMPORTANT: Trigger methods MUST respond with a boolean
    ///
    /// ```bash
    /// near call manager_v1.croncat.testnet create_trigger '{"contract_id": "counter.in.testnet","function_id": "increment","arguments":"","task_hash":""}' --accountId YOU.testnet
    /// ```
    #[payable]
    pub fn create_trigger(
        &mut self,
        contract_id: ValidAccountId,
        function_id: String,
        task_hash: Base64VecU8,
        arguments: Option<Base64VecU8>,
    ) -> Base64VecU8 {
        // No adding triggers while contract is paused
        assert_eq!(self.paused, false, "Create trigger paused");
        // Check attached deposit includes trigger_storage_usage
        assert!(
            env::attached_deposit() >= self.trigger_storage_usage as u128,
            "Trigger storage payment of {} required",
            self.trigger_storage_usage
        );
        // prevent dumb mistakes
        assert!(contract_id.to_string().len() > 0, "Contract ID missing");
        assert!(function_id.len() > 0, "Function ID missing");
        assert!(task_hash.0.len() > 0, "Task Hash missing");
        assert_ne!(
            contract_id.clone().to_string(),
            env::current_account_id(),
            "Trigger cannot call self"
        );

        // Confirm owner of task is same
        let task = self.tasks.get(&task_hash.0).expect("No task found");
        assert_eq!(
            task.owner_id,
            env::predecessor_account_id(),
            "Must be task owner"
        );

        let item = Trigger {
            owner_id: env::predecessor_account_id(),
            contract_id: contract_id.into(),
            function_id,
            task_hash,
            arguments: arguments.unwrap_or_else(|| Base64VecU8::from(vec![])),
        };

        let trigger_hash = self.get_trigger_hash(&item);

        // Add trigger to catalog
        assert!(
            self.triggers.insert(&trigger_hash, &item).is_none(),
            "Trigger already exists"
        );

        Base64VecU8::from(trigger_hash)
    }

    /// Deletes a task in its entirety, returning any remaining balance to task owner.
    ///
    /// ```bash
    /// near call manager_v1.croncat.testnet remove_trigger '{"trigger_hash": ""}' --accountId YOU.testnet
    /// ```
    pub fn remove_trigger(&mut self, trigger_hash: Base64VecU8) {
        let hash = trigger_hash.0;
        let trigger = self.triggers.get(&hash).expect("No task found by hash");

        assert_eq!(
            trigger.owner_id,
            env::predecessor_account_id(),
            "Only owner can remove their trigger."
        );

        // If owner, allow to remove task
        self.triggers
            .remove(&hash)
            .expect("No trigger found by hash");

        // Refund trigger storage
        Promise::new(trigger.owner_id).transfer(self.trigger_storage_usage as u128);
    }

    /// Get the hash of a trigger based on parameters
    pub fn get_trigger_hash(&self, item: &Trigger) -> Vec<u8> {
        // Generate hash, needs to be from known values so we can reproduce the hash without storing
        let input = format!(
            "{:?}{:?}{:?}{:?}{:?}",
            item.contract_id, item.function_id, item.task_hash, item.owner_id, item.arguments
        );
        env::sha256(input.as_bytes())
    }

    /// Returns trigger data
    ///
    /// ```bash
    /// near view manager_v1.croncat.testnet get_triggers '{"from_index": 0, "limit": 10}'
    /// ```
    pub fn get_triggers(
        &self,
        from_index: Option<U64>,
        limit: Option<U64>,
    ) -> Vec<TriggerHumanFriendly> {
        let mut ret: Vec<TriggerHumanFriendly> = Vec::new();
        let mut start = 0;
        let mut end = 10;
        if let Some(from_index) = from_index {
            start = from_index.0;
        }
        if let Some(limit) = limit {
            end = u64::min(start + limit.0, self.tasks.len());
        }

        // Return all tasks within range
        let keys = self.triggers.keys_as_vector();
        for i in start..end {
            if let Some(trigger_hash) = keys.get(i) {
                if let Some(trigger) = self.triggers.get(&trigger_hash) {
                    ret.push(TriggerHumanFriendly {
                        owner_id: trigger.owner_id.clone(),
                        contract_id: trigger.contract_id.clone(),
                        function_id: trigger.function_id.clone(),
                        arguments: trigger.arguments.clone(),
                        task_hash: trigger.task_hash.clone(),
                        hash: Base64VecU8::from(self.get_trigger_hash(&trigger)),
                    });
                }
            }
        }
        ret
    }

    /// Returns trigger
    ///
    /// ```bash
    /// near view manager_v1.croncat.testnet get_trigger '{"trigger_hash": "..."}'
    /// ```
    pub fn get_trigger(&self, trigger_hash: Base64VecU8) -> TriggerHumanFriendly {
        let trigger = self
            .triggers
            .get(&trigger_hash.0)
            .expect("No trigger found");

        TriggerHumanFriendly {
            owner_id: trigger.owner_id.clone(),
            contract_id: trigger.contract_id.clone(),
            function_id: trigger.function_id.clone(),
            arguments: trigger.arguments.clone(),
            task_hash: trigger.task_hash.clone(),
            hash: Base64VecU8::from(self.get_trigger_hash(&trigger)),
        }
    }

    /// !IMPORTANT!:: BETA FEATURE!!!!!!!!!
    /// Allows agents to check if a view method should trigger a task immediately
    ///
    /// TODO:
    /// - Check for range hash
    /// - Loop range to find view BOOL TRUE
    /// - Get task details
    /// - Execute task
    ///
    /// ```bash
    /// near call manager_v1.croncat.testnet proxy_conditional_call '{"trigger_hash": ""}' --accountId YOU.testnet
    /// ```
    pub fn proxy_conditional_call(&mut self, trigger_hash: Base64VecU8) {
        // No adding tasks while contract is paused
        assert_eq!(self.paused, false, "Task execution paused");

        // only registered agent signed, because micropayments will benefit long term
        let agent_opt = self.agents.get(&env::predecessor_account_id());
        if agent_opt.is_none() {
            env::panic(b"Agent not registered");
        }

        // TODO: Think about agent rewards - as they could pay for a failed CB
        let trigger = self
            .triggers
            .get(&trigger_hash.into())
            .expect("No trigger found by hash");

        // TODO: check the task actually exists

        // Make sure this isnt calling manager
        assert_ne!(
            trigger.contract_id.clone().to_string(),
            env::current_account_id(),
            "Trigger cannot call self"
        );

        // Call external contract with task variables
        let promise_first = env::promise_create(
            trigger.contract_id.clone(),
            &trigger.function_id.as_bytes(),
            trigger.arguments.0.as_slice(),
            NO_DEPOSIT,
            VIEW_CALL_GAS,
        );
        let promise_second = env::promise_then(
            promise_first,
            env::current_account_id(),
            b"proxy_conditional_callback",
            json!({
                "task_hash": trigger.task_hash,
                "agent_id": &env::predecessor_account_id(),
            })
            .to_string()
            .as_bytes(),
            NO_DEPOSIT,
            GAS_FOR_CALLBACK,
        );
        env::promise_return(promise_second);
    }

    /// !IMPORTANT!:: BETA FEATURE!!!!!!!!!
    /// Callback, if response is TRUE, then do the actual proxy call
    #[private]
    pub fn proxy_conditional_callback(&mut self, task_hash: Base64VecU8, agent_id: AccountId) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "Expected 1 promise result."
        );
        let mut agent = self.agents.get(&agent_id).expect("Agent not found");
        match env::promise_result(0) {
            PromiseResult::NotReady => {
                unreachable!()
            }
            PromiseResult::Successful(trigger_result) => {
                let result: CroncatTriggerResponse = serde_json::de::from_slice(&trigger_result)
                    .expect("Could not get result from trigger");

                // TODO: Refactor to re-used method
                if result.0 {
                    let mut task = self
                        .tasks
                        .get(&task_hash.clone().into())
                        .expect("No task found by hash");

                    // Fee breakdown:
                    // - Used Gas: Task Txn Fee Cost
                    // - Agent Fee: Incentivize Execution SLA
                    //
                    // Task Fee Examples:
                    // Total Fee = Gas Fee + Agent Fee
                    // Total Balance = Task Deposit + Total Fee
                    //
                    // NOTE: Gas cost includes the cross-contract call & internal logic of this contract.
                    // Direct contract gas fee will be lower than task execution costs, however
                    // we require the task owner to appropriately estimate gas for overpayment.
                    // The gas overpayment will also accrue to the agent since there is no way to read
                    // how much gas was actually used on callback.
                    let call_fee_used = u128::from(task.gas) * self.gas_price;
                    let call_total_fee = call_fee_used + self.agent_fee;
                    let call_total_balance = task.deposit.0 + call_total_fee;

                    // Update agent storage
                    // Increment agent reward & task count
                    // Reward for agent MUST include the amount of gas used as a reimbursement
                    agent.balance = U128::from(agent.balance.0 + call_total_fee);
                    agent.total_tasks_executed = U128::from(agent.total_tasks_executed.0 + 1);
                    self.available_balance = self.available_balance - call_total_fee;

                    // Reset missed slot, if any
                    if agent.last_missed_slot != 0 {
                        agent.last_missed_slot = 0;
                    }
                    self.agents.insert(&env::signer_account_id(), &agent);

                    // Decrease task balance, Update task storage
                    task.total_deposit = U128::from(task.total_deposit.0 - call_total_balance);
                    self.tasks.insert(&task_hash.into(), &task);

                    // Call external contract with task variables
                    let promise_first = env::promise_create(
                        task.contract_id.clone(),
                        &task.function_id.as_bytes(),
                        // TODO: support CroncatTriggerResponse optional view arguments
                        task.arguments.0.as_slice(),
                        task.deposit.0,
                        task.gas,
                    );

                    env::promise_return(promise_first);
                } else {
                    log!("Trigger returned false");
                }
            }
            PromiseResult::Failed => {
                // Problem with the creation transaction, reward money has been returned to this contract.
                log!("Trigger call failed");
                self.send_base_agent_reward(agent);
            }
        }
    }
}

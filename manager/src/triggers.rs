use near_sdk::serde_json;
use crate::*;

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

#[near_bindgen]
impl Contract {
    /// Configure a VIEW call to map to a task, allowing IFTTT functionality
    /// IMPORTANT: Trigger methods MUST respond with a boolean
    ///
    /// ```bash
    /// near call cron.testnet create_trigger '{"contract_id": "counter.in.testnet","function_id": "increment","arguments":"","task_hash":""}' --accountId YOU.testnet
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
        assert!(env::attached_deposit() >= self.trigger_storage_usage as u128, "Trigger storage payment of {} required", self.trigger_storage_usage);

        let item = Trigger {
            owner_id: env::predecessor_account_id(),
            contract_id: contract_id.into(),
            function_id,
            task_hash,
            arguments: arguments.unwrap_or_else(|| Base64VecU8::from(vec![])),
        };

        let trigger_hash = self.trigger_hash(&item);

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
    /// near call cron.testnet remove_trigger '{"trigger_hash": ""}' --accountId YOU.testnet
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
        self.triggers.remove(&hash).expect("No trigger found by hash");

        // Refund trigger storage
        Promise::new(trigger.owner_id).transfer(self.trigger_storage_usage as u128);
    }

    // /// Method for agent to view & evaluate range of view calls
    // /// Callable by anyone, but only active agents can execute
    // // Based on active agent index, agent is responsible to a range of view calls, 
    // // if any evaluate to true return the range index, as long as the pagination sort doesnt change, this could work. 
    // // Does knowing the RANGE give enough information for agent to skew execution?
    // // Bonus here is the view ranges can scale with the view needs, as agents could get assigned multiple ranges potentially that are async RPC calls
    // pub fn proxy_view(&self) -> Base64VecU8 {
    //   let view_results: Vec<bool> = Vec::new();
    //   // TODO:
    //   // get a range of view triggers
    //   // Loop and call each
    //   // Create a that represents if the range has any TRUE results
    //   let start = 0; // TODO: Change to better range management
    //   let end = self.triggers.len();

    //   // Return all tasks within range
    //   let keys = self.triggers.keys_as_vector();
    //   for i in start..end {
    //     if let Some(trigger_hash) = keys.get(i) {
    //       if let Some(trigger) = self.triggers.get(&trigger_hash) {
    //         if let Some(task) = self.tasks.get(&trigger.task_hash.0.as_slice()) {
    //           let promise_first = env::promise_create(
    //             trigger.contract_id.clone(),
    //             &trigger.function_id.as_bytes(),
    //             trigger.arguments.0.as_slice(),
    //             NO_DEPOSIT,
    //             VIEW_CALL_GAS,
    //           );
    //           let promise_second = env::promise_then(
    //             promise_first,
    //             task.contract_id.clone(),
    //             &task.function_id.as_bytes(),
    //             task.arguments.0.as_slice(),
    //             task.deposit.0,
    //             task.gas,
    //           );
    //           env::promise_return(promise_second);
    //           env::promise_batch_create(account_id)
    //         }
    //       }
    //     }
    //   }

    //   let input = format!(
    //       "{:?}{:?}{:?}{:?}",
    //       item.contract_id, item.function_id, item.task_hash, item.owner_id
    //   );
    //   let hash = env::sha256(input.as_bytes());
    //   Base64VecU8::from(hash)
    // }

    /// Method for agent to view & evaluate range of view calls
    /// DEMO ONLY
    pub fn proxy_view(&self) {
      // let view_results: Vec<bool> = Vec::new();
      // // TODO:
      // // get a range of view triggers
      // // Loop and call each
      // // Create a that represents if the range has any TRUE results
      // let start = 0; // TODO: Change to better range management
      // let end = self.triggers.len();

      // Return all tasks within range
      let keys = self.triggers.keys_as_vector();
      if let Some(trigger_hash) = keys.get(0) {
        if let Some(trigger) = self.triggers.get(&trigger_hash) {
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
            b"callback_for_proxy_view",
            json!({
              "task_hash": trigger.task_hash
            })
            .to_string()
            .as_bytes(),
            0,
            GAS_FOR_CALLBACK,
          );
          env::promise_return(promise_second);
        }
      }
    }

    /// NOTE: Testing
    #[private]
    pub fn callback_for_proxy_view(&mut self, task_hash: Base64VecU8) -> bool {
      assert_eq!(
        env::promise_results_count(),
        1,
        "Expected 1 promise result."
      );
      // TODO: Could loop the promise results here
      match env::promise_result(0) {
        PromiseResult::NotReady => {
          unreachable!()
        }
        PromiseResult::Successful(trigger_result) => {
          log!("trigger_result {:?}", &trigger_result);
          let result: bool = serde_json::de::from_slice(&trigger_result)
            .expect("Could not get result from trigger");
          log!("results {:?} {:?}", result, &task_hash);

          result
        }
        PromiseResult::Failed => {
          // Problem with the creation transaction, reward money has been returned to this contract.
          log!("No results");
          false
        }
      }
    }

    // /// Allows agents to check if a view method should trigger a task immediately
    // ///
    // /// TODO:
    // /// - Check for range hash 
    // /// - Loop range to find view BOOL TRUE
    // /// - Get task details
    // /// - Execute task
    // ///
    // /// ```bash
    // /// near call cron.testnet proxy_call --accountId YOU.testnet
    // /// ```
    // pub fn proxy_call_conditional(&mut self, range_hash: Base64VecU8) {
    //     // No adding tasks while contract is paused
    //     assert_eq!(self.paused, false, "Task execution paused");

    //     // only registered agent signed, because micropayments will benefit long term
    //     let agent_opt = self.agents.get(&env::predecessor_account_id());
    //     if agent_opt.is_none() {
    //         env::panic(b"Agent not registered");
    //     }
    //     let mut agent = agent_opt.unwrap();

    //     // Get current slot based on block or timestamp
    //     let current_slot = self.get_slot_id(None);

    //     // get task based on current slot
    //     // priority goes to tasks that have fallen behind (using floor key)
    //     let (slot_opt, slot_ballpark) = if let Some(k) = self.slots.floor_key(&current_slot) {
    //         (self.slots.get(&k), k)
    //     } else {
    //         (self.slots.get(&current_slot), current_slot)
    //     };

    //     let mut slot_data = slot_opt.expect("No tasks found in slot");

    //     // Check if agent has exceeded their slot task allotment
    //     // TODO: An agent can check to execute IF slot is +1 and their index is within range???
    //     let (can_execute, current_agent_index, _) =
    //         self.check_agent_can_execute(env::predecessor_account_id(), slot_data.len() as u64);
    //     assert!(can_execute, "Agent has exceeded execution for this slot");
    //     // Rotate agent index
    //     if self.agent_active_index as u64 == self.agent_active_queue.len().saturating_sub(1) {
    //         self.agent_active_index = 0;
    //     } else if self.agent_active_queue.len() > 1 {
    //         // Only change the index IF there are more than 1 agents ;)
    //         self.agent_active_index += 1;
    //     }
    //     // IF previous agent missed, then store their slot missed. We know this is true IF this slot is using slot_ballpark
    //     // NOTE: While this isnt perfect, the eventual outcome is fine.
    //     //       If agent gets ticked as "missed" for maximum of 1 slot, then fixes the situation on next round.
    //     //       If agent truly misses enough slots, they will skip their chance to reset missed slot count and be dropped.
    //     if slot_ballpark < current_slot && self.agent_active_queue.len() > 1 {
    //         // wrap around logic for non-overflow index
    //         // if only 1 agent, dont do anything
    //         let missed_agent_index = if current_agent_index == 0 {
    //             self.agent_active_queue.len()
    //         } else {
    //             current_agent_index - 1
    //         };
    //         let missed_agent_id = self.agent_active_queue.get(missed_agent_index);

    //         if let Some(missed_agent_id) = missed_agent_id {
    //             let missed_agent = self.agents.get(&missed_agent_id);

    //             // confirm we should update missed slot, ONLY if the slot id is 0, otherwise the agent has not reset the count and we shouldnt mess with it.
    //             if let Some(missed_agent) = missed_agent {
    //                 let mut m_agent = missed_agent;
    //                 if m_agent.last_missed_slot == 0 {
    //                     m_agent.last_missed_slot = slot_ballpark;
    //                     // update storage
    //                     self.agents.insert(&missed_agent_id, &m_agent);
    //                 }
    //             }
    //         }
    //     }

    //     // Get a single task hash, then retrieve task details
    //     let hash = slot_data.pop().expect("No tasks available");

    //     // After popping, ensure state is rewritten back
    //     if slot_data.is_empty() {
    //         // Clean up slot if no more data
    //         self.slots.remove(&slot_ballpark);
    //         // log!("Slot {} cleaned", &slot_ballpark);
    //     } else {
    //         self.slots.insert(&slot_ballpark, &slot_data);
    //     }

    //     let mut task = self.tasks.get(&hash).expect("No task found by hash");

    //     // Fee breakdown:
    //     // - Used Gas: Task Txn Fee Cost
    //     // - Agent Fee: Incentivize Execution SLA
    //     //
    //     // Task Fee Examples:
    //     // Total Fee = Gas Fee + Agent Fee
    //     // Total Balance = Task Deposit + Total Fee
    //     //
    //     // NOTE: Gas cost includes the cross-contract call & internal logic of this contract.
    //     // Direct contract gas fee will be lower than task execution costs, however
    //     // we require the task owner to appropriately estimate gas for overpayment.
    //     // The gas overpayment will also accrue to the agent since there is no way to read
    //     // how much gas was actually used on callback.
    //     let call_fee_used = u128::from(task.gas) * self.gas_price;
    //     let call_total_fee = call_fee_used + self.agent_fee;
    //     let call_total_balance = task.deposit.0 + call_total_fee;

    //     // Update agent storage
    //     // Increment agent reward & task count
    //     // Reward for agent MUST include the amount of gas used as a reimbursement
    //     agent.balance = U128::from(agent.balance.0 + call_total_fee);
    //     agent.total_tasks_executed = U128::from(agent.total_tasks_executed.0 + 1);
    //     self.available_balance = self.available_balance - call_total_fee;

    //     // Reset missed slot, if any
    //     if agent.last_missed_slot != 0 {
    //         agent.last_missed_slot = 0;
    //     }
    //     self.agents.insert(&env::signer_account_id(), &agent);

    //     // Decrease task balance, Update task storage
    //     task.total_deposit = U128::from(task.total_deposit.0 - call_total_balance);
    //     self.tasks.insert(&hash, &task);

    //     // Call external contract with task variables
    //     let promise_first = env::promise_create(
    //         task.contract_id.clone(),
    //         &task.function_id.as_bytes(),
    //         task.arguments.0.as_slice(),
    //         task.deposit.0,
    //         task.gas,
    //     );

    //     env::promise_return(promise_first);
    // }
}

// Internal methods
impl Contract {
    /// Get the hash of a trigger based on parameters
    fn trigger_hash(&self, item: &Trigger) -> Vec<u8> {
        // Generate hash, needs to be from known values so we can reproduce the hash without storing
        let input = format!(
            "{:?}{:?}{:?}{:?}",
            item.contract_id, item.function_id, item.task_hash, item.owner_id
        );
        env::sha256(input.as_bytes())
    }
}


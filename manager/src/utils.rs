use crate::*;

#[near_bindgen]
impl Contract {
    // NOTE: For large state transitions, needs to be able to migrate over paginated sets?
    /// Migrate State
    /// Safely upgrade contract storage
    ///
    /// ```bash
    /// near call cron.testnet migrate_state --accountId cron.testnet
    /// ```
    #[init(ignore_state)]
    #[private]
    pub fn migrate_state() -> Self {
        // Deserialize the state using the old contract structure.
        let old_contract: Contract = env::state_read().expect("Old state doesn't exist");
        // Verify that the migration can only be done by the owner.
        // This is not necessary, if the upgrade is done internally.
        assert_eq!(
            &env::predecessor_account_id(),
            &old_contract.owner_id,
            "Can only be called by the owner"
        );

        // Create the new contract using the data from the old contract.
        // Contract { owner_id: old_contract.owner_id, data: old_contract.data, new_data }
        Contract {
            paused: false,
            owner_id: old_contract.owner_id,
            tasks: old_contract.tasks,
            slots: old_contract.slots,
            slot_granularity: old_contract.slot_granularity,
            available_balance: old_contract.available_balance,
            staked_balance: old_contract.staked_balance,
            agent_fee: old_contract.agent_fee,
            gas_price: old_contract.gas_price,
            proxy_callback_gas: old_contract.proxy_callback_gas,
            agents: old_contract.agents,
            agent_storage_usage: old_contract.agent_storage_usage,
            agent_active_queue: Vector::new(StorageKeys::AgentsActive),
            agent_pending_queue: Vector::new(StorageKeys::AgentsPending),
            agent_task_ratio: [1, 2],
            agent_active_index: 0,
            agents_eject_threshold: 10,
        }
    }

    /// Tick: Cron Manager Heartbeat
    /// Used to manage agents, manage internal use of funds
    ///
    /// Return operations balances, for external on-chain contract monitoring
    ///
    /// near call cron.testnet tick '{}'
    pub fn tick(&mut self) {
        // TBD: Internal staking management
        log!(
            "Balances [Operations, Treasury]:  [{},{}]",
            self.available_balance,
            self.staked_balance
        );

        // execute agent management every tick so we can allow coming/going of agents without each agent paying to manage themselves
        // NOTE: the agent CAN pay to execute "tick" method if they are anxious to become an active agent. The most they can query is every 10s.
        self.manage_agents();
    }

    /// Manage agents
    fn manage_agents(&mut self) {
        let current_slot = self.get_slot_id(None);

        // Loop all agents to assess if really active
        // Why the copy here? had to get a mutable reference from immutable self instance
        let mut bad_agents: Vec<AccountId> = Vec::from(self.agent_active_queue.to_vec());
        bad_agents.retain(|agent_id| {
            let _agent = self.agents.get(&agent_id);

            if let Some(_agent) = _agent {
                let last_slot = u128::from(_agent.last_missed_slot);

                // Check if any agents need to be ejected, looking at previous task slot and current
                // LOGIC: If agent misses X number of slots, eject!
                if current_slot
                    > last_slot + (self.agents_eject_threshold * u128::from(self.slot_granularity))
                {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        });

        // EJECT!
        // Dont eject if only 1 agent remaining... so sad. no lonely allowed.
        if self.agent_active_queue.len() > 2 {
            for id in bad_agents {
                self.exit_agent(Some(id), Some(true));
            }
        }

        // Get data needed to check for agent<>task ratio
        let total_tasks = self.tasks.len();
        let total_agents = self.agent_active_queue.len();
        let [agent_amount, task_amount] = self.agent_task_ratio;

        // no panic returns. safe-guard from idiot ratios.
        if total_tasks == 0 || total_agents == 0 {
            return;
        }
        if agent_amount == 0 || task_amount == 0 {
            return;
        }
        let ratio = task_amount.div_euclid(agent_amount);
        let total_available_agents = total_tasks.div_euclid(ratio);

        // Check if there are more tasks to allow a new agent
        if total_available_agents > total_agents {
            // There's enough tasks to support another agent, check if we have any pending
            if self.agent_pending_queue.len() > 0 {
                // FIFO grab pending agents
                let agent_id = self.agent_pending_queue.swap_remove(0);
                if let Some(mut agent) = self.agents.get(&agent_id) {
                    agent.status = agent::AgentStatus::Active;
                    self.agents.insert(&agent_id, &agent);
                    self.agent_active_queue.push(&agent_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::json_types::ValidAccountId;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    const BLOCK_START_TS: u64 = 1633759320000000000;

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

    // TODO: Add test for checking pending agent here.
    #[test]
    fn test_tick() {
        let mut context = get_context(accounts(1));
        testing_env!(context.is_view(false).build());
        let mut contract = Contract::new();
        testing_env!(context.is_view(true).build());
        testing_env!(context
            .is_view(false)
            .block_timestamp(1633759440000000000)
            .build());
        contract.tick();
        testing_env!(context
            .is_view(false)
            .block_timestamp(1633760160000000000)
            .build());
        contract.tick();
        testing_env!(context
            .is_view(false)
            .block_timestamp(1633760460000000000)
            .build());
        testing_env!(context.is_view(true).build());
    }
}

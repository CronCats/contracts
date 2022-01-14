use crate::*;

#[near_bindgen]
impl Contract {
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
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::{AccountId, PublicKey};

    const BLOCK_START_TS: u64 = 1633759320000000000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .signer_account_pk(
                PublicKey::from_str("ed25519:4ZhGmuKTfQn9ZpHCQVRwEr4JnutL8Uu3kArfxEqksfVM")
                    .unwrap(),
            )
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

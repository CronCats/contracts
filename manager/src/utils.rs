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
            bps_block: old_contract.bps_block,
            bps_timestamp: old_contract.bps_timestamp,
            tasks: old_contract.tasks,
            slots: old_contract.slots,
            slot_granularity: old_contract.slot_granularity,
            active_slot: ActiveSlot {
                id: env::block_index(),
                total_tasks: 0,
            },
            available_balance: old_contract.available_balance,
            staked_balance: old_contract.staked_balance,
            agent_fee: old_contract.agent_fee,
            gas_price: old_contract.gas_price,
            proxy_callback_gas: old_contract.proxy_callback_gas,
            agents: old_contract.agents,
            agent_storage_usage: old_contract.agent_storage_usage,
            agent_pending_queue: LookupSet::new(StorageKeys::AgentsPending),
            agent_task_ratio: [1, 2],
            agents_total: 0,
        }
    }

    /// Tick: Cron Manager Heartbeat
    /// Used to aid computation of blocks per second, manage internal use of funds
    /// NOTE: This is a small array, allowing the adjustment of the previous block in the past
    /// so the block tps average is always using more block distance than "now", ideally ~1000 blocks
    ///
    /// near call cron.testnet tick '{}'
    pub fn tick(&mut self) {
        let prev_block = self.bps_block[0];
        let prev_timestamp = self.bps_timestamp[0];

        // Check that we dont allow 0 BPS
        assert!(prev_block + 10 < env::block_index(), "Tick triggered too soon");

        self.bps_block[0] = env::block_index();
        self.bps_block[1] = prev_block;
        self.bps_timestamp[0] = env::block_timestamp();
        self.bps_timestamp[1] = prev_timestamp;

        // TBD: Internal staking management
        log!(
            "Balances: Available {}, Staked {}",
            self.available_balance,
            self.staked_balance
        );
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
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
    fn test_tick() {
        let mut context = get_context(accounts(1));
        testing_env!(context.is_view(false).build());
        let mut contract = Contract::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.bps_block[0], 52201040);
        testing_env!(context.is_view(false).block_index(52201240).build());
        contract.tick();
        testing_env!(context.is_view(false).block_index(52207040).build());
        contract.tick();
        testing_env!(context.is_view(false).block_index(52208540).build());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.bps_block[0], 52207040);
        assert_eq!(contract.bps_block[1], 52201240);
    }
}

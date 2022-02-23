use crate::*;

#[near_bindgen]
impl Contract {
    /// Changes core configurations
    /// Should only be updated by owner -- in best case DAO based :)
    pub fn update_settings(
        &mut self,
        owner_id: Option<AccountId>,
        slot_granularity: Option<u64>,
        paused: Option<bool>,
        agent_fee: Option<U128>,
        gas_price: Option<U128>,
        proxy_callback_gas: Option<U64>,
        agent_task_ratio: Option<Vec<U64>>,
        agents_eject_threshold: Option<U128>,
        treasury_id: Option<AccountId>,
    ) {
        assert_eq!(
            self.owner_id,
            env::predecessor_account_id(),
            "Must be owner"
        );

        // BE CAREFUL!
        if let Some(owner_id) = owner_id {
            self.owner_id = owner_id;
        }
        if let Some(treasury_id) = treasury_id {
            self.treasury_id = Some(treasury_id);
        }

        if let Some(slot_granularity) = slot_granularity {
            self.slot_granularity = slot_granularity;
        }
        if let Some(paused) = paused {
            self.paused = paused;
        }
        if let Some(gas_price) = gas_price {
            self.gas_price = gas_price.0;
        }
        if let Some(proxy_callback_gas) = proxy_callback_gas {
            self.proxy_callback_gas = proxy_callback_gas.0;
        }
        if let Some(agent_fee) = agent_fee {
            self.agent_fee = agent_fee.0;
        }
        if let Some(agent_task_ratio) = agent_task_ratio {
            self.agent_task_ratio = [agent_task_ratio[0].0, agent_task_ratio[1].0];
        }
        if let Some(agents_eject_threshold) = agents_eject_threshold {
            self.agents_eject_threshold = agents_eject_threshold.0;
        }
    }

    /// Allows admin to calculate internal balances
    /// Returns surplus and rewards balances
    /// Can be used to measure how much surplus is remaining for staking / etc
    #[private]
    pub fn calc_balances(&mut self) -> (U128, U128) {
        let base_balance = BASE_BALANCE; // safety overhead
        let storage_balance = env::storage_byte_cost().saturating_mul(env::storage_usage() as u128);

        // Using storage + threshold as the start for how much balance is required
        let required_balance = base_balance.saturating_add(storage_balance);
        let mut total_task_balance: Balance = 0;
        let mut total_reward_balance: Balance = 0;

        // Loop all tasks and add
        for (_, t) in self.tasks.iter() {
            total_task_balance = total_task_balance.saturating_add(t.total_deposit.0);
        }

        // Loop all agents rewards and add
        for a in self.agent_active_queue.iter() {
            if let Some(agent) = self.agents.get(&a) {
                total_reward_balance = total_reward_balance.saturating_add(agent.balance.0);
            }
        }

        let total_available_balance: Balance =
            total_task_balance.saturating_add(total_reward_balance);

        // Calculate surplus, which could be used for staking
        let surplus = u128::max(total_available_balance.saturating_sub(required_balance), 0);
        log!("Stakeable surplus {}", surplus);

        // update internal values
        self.available_balance =
            u128::max(total_available_balance.saturating_sub(required_balance), 0);

        // Return surplus value in case we want to trigger staking based off outcome
        (U128::from(surplus), U128::from(total_reward_balance))
    }

    /// Move Balance
    /// Allows owner to move balance to DAO or to let treasury transfer to itself only.
    pub fn move_balance(&mut self, amount: U128, account_id: AccountId) -> Promise {
        // Check if is owner OR the treasury account
        let transfer_warning = b"Not approved for transfer";
        if let Some(treasury_id) = self.treasury_id.clone() {
            if treasury_id != env::predecessor_account_id()
                && self.owner_id != env::predecessor_account_id()
            {
                env::panic(transfer_warning);
            }
        } else if self.owner_id != env::predecessor_account_id() {
            env::panic(transfer_warning);
        }
        // for now, only allow movement of funds between owner and treasury
        let check_account = self.treasury_id.clone().unwrap_or(self.owner_id.clone());
        if check_account != account_id.clone() {
            env::panic(b"Cannot move funds to this account");
        }
        // Check that the amount is not larger than available
        let (_, _, _, surplus) = self.get_balances();
        assert!(amount.0 < surplus.0, "Amount is too high");

        // transfer
        // NOTE: Not updating available balance, as we are simply allowing surplus transfer only
        Promise::new(account_id).transfer(amount.0)
    }

    /// Allows admin to remove slot data, in case a task gets stuck due to missed exits
    pub fn remove_slot_owner(&mut self, slot: U128) {
        // assert_eq!(
        //     self.owner_id,
        //     env::predecessor_account_id(),
        //     "Must be owner"
        // );
        assert_eq!(
            env::current_account_id(),
            env::predecessor_account_id(),
            "Must be owner"
        );
        self.slots.remove(&slot.0);
    }

    /// Deletes a task in its entirety, returning any remaining balance to task owner.
    ///
    /// ```bash
    /// near call manager_v1.croncat.testnet remove_task_owner '{"task_hash": ""}' --accountId YOU.testnet
    /// ```
    #[private]
    pub fn remove_task_owner(&mut self, task_hash: Base64VecU8) {
        let hash = task_hash.0;
        self.tasks.get(&hash).expect("No task found by hash");

        // If owner, allow to remove task
        self.exit_task(hash);
    }

    /// Deletes a trigger in its entirety, only by owner.
    ///
    /// ```bash
    /// near call manager_v1.croncat.testnet remove_trigger_owner '{"trigger_hash": ""}' --accountId YOU.testnet
    /// ```
    #[private]
    pub fn remove_trigger_owner(&mut self, trigger_hash: Base64VecU8) {
        self.triggers.remove(&trigger_hash.0).expect("No trigger found by hash");
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
    #[should_panic(expected = "Must be owner")]
    fn test_update_settings_fail() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.slot_granularity, SLOT_GRANULARITY);

        testing_env!(context
            .is_view(false)
            .signer_account_id(accounts(3))
            .predecessor_account_id(accounts(3))
            .build());
        contract.update_settings(None, Some(10), None, None, None, None, None, None, None);
    }

    #[test]
    fn test_update_settings() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.slot_granularity, SLOT_GRANULARITY);

        testing_env!(context.is_view(false).build());
        contract.update_settings(
            None,
            Some(10),
            Some(true),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.slot_granularity, 10);
        assert_eq!(contract.paused, true);
    }

    #[test]
    fn test_update_settings_agent_ratio() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.slot_granularity, SLOT_GRANULARITY);

        testing_env!(context.is_view(false).build());
        contract.update_settings(
            None,
            None,
            Some(true),
            None,
            None,
            None,
            Some(vec![U64(2), U64(5)]),
            None,
            None,
        );
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.agent_task_ratio[0], 2);
        assert_eq!(contract.agent_task_ratio[1], 5);
        assert_eq!(contract.paused, true);
    }

    #[test]
    fn test_calc_balances() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();
        let base_agent_storage: u128 = 2260000000000000000000;
        contract.calc_balances();

        testing_env!(context
            .is_view(false)
            .attached_deposit(ONE_NEAR * 5)
            .build());
        contract.create_task(
            accounts(3),
            "increment".to_string(),
            "0 0 */1 * * *".to_string(),
            Some(true),
            Some(U128::from(ONE_NEAR)),
            Some(200),
            None,
        );
        contract.register_agent(Some(accounts(1)));
        testing_env!(context.is_view(false).build());

        // recalc the balances
        let (surplus, rewards) = contract.calc_balances();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.available_balance, 0);
        assert_eq!(surplus.0, 0);
        assert_eq!(rewards.0, base_agent_storage);
    }

    #[test]
    fn test_move_balance() {
        let mut context = get_context(accounts(1));
        testing_env!(context.is_view(false).build());
        let mut contract = Contract::new();
        contract.calc_balances();
        contract.move_balance(U128::from(ONE_NEAR / 2), accounts(1).to_string());
        testing_env!(context.is_view(true).build());

        let (_, _, _, surplus) = contract.get_balances();
        assert_eq!(surplus.0, 91928000000000000000000000);
    }
}

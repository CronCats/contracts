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
        contract.update_settings(None, Some(10), None, None, None, None, None);
    }

    #[test]
    fn test_update_settings() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.slot_granularity, SLOT_GRANULARITY);

        testing_env!(context.is_view(false).build());
        contract.update_settings(None, Some(10), Some(true), None, None, None, None);
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
        );
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.agent_task_ratio[0], 2);
        assert_eq!(contract.agent_task_ratio[1], 5);
        assert_eq!(contract.paused, true);
    }
}

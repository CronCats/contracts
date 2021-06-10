use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds, StorageManagement};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{assert_one_yocto, env, log, AccountId, Balance, Promise};
use crate::CronManager;

impl CronManager {
    fn internal_storage_balance_of(&self, account_id: &AccountId) -> Option<StorageBalance> {
        if self.agents.contains_key(account_id) {
            // The "available" balance is always zero because the storage isn't
            // variable for this contract.
            Some(StorageBalance { total: self.storage_balance_bounds().min, available: 0.into() })
        } else {
            None
        }
    }
}

impl StorageManagement for CronManager {
    // `registration_only` doesn't affect the implementation here, as there's no need to add additional
    // storage, so there's only one balance to attach.
    #[allow(unused_variables)]
    fn storage_deposit(
        &mut self,
        account_id: Option<ValidAccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.register_agent(account_id.clone(), None);
        let account_id = account_id.map(|a| a.into()).unwrap_or_else(|| env::predecessor_account_id());
        self.internal_storage_balance_of(&account_id).unwrap()
    }

    /// While storage_withdraw normally allows the caller to retrieve `available` balance, this
    /// contract sets storage_balance_bounds.min = storage_balance_bounds.max,
    /// which means available balance will always be 0. So this implementation:
    /// * panics if `amount > 0`
    /// * never transfers Ⓝ to caller
    /// * returns a `storage_balance` struct if `amount` is 0
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        assert_one_yocto();
        let predecessor = env::predecessor_account_id();
        if let Some(storage_balance) = self.internal_storage_balance_of(&predecessor) {
            match amount {
                Some(amount) if amount.0 > 0 => {
                    let panic_msg = format!("The amount is greater than the available storage balance. Remember there's a minimum balance needed for an agent's storage. That minimum is {}. To unregister an agent, use the 'unregister_agent' or 'storage_unregister' with the 'force' option.", self.agent_storage_usage);
                    env::panic(panic_msg.as_bytes());
                }
                _ => storage_balance,
            }
        } else {
            env::panic(
                format!("The account {} is not registered", &predecessor).as_bytes(),
            );
        }
    }

    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        let force = force.unwrap_or(false);
        if let Some(agent) = self.agents.get(&account_id) {
            let balance = agent.balance.0;
            if balance == 0 || force {
                self.agents.remove(&account_id);
                // We add 1 to reimburse for the 1 yoctoⓃ used to call this method
                Promise::new(account_id.clone()).transfer(balance + 1);
                true
            } else {
                env::panic(b"Can't unregister the agent with the positive balance. Must use the 'force' parameter if desired.")
            }
        } else {
            log!("The agent {} is not registered", &account_id);
            false
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        let required_storage_balance = Balance::from(self.agent_storage_usage) * env::storage_byte_cost();
        StorageBalanceBounds {
            min: required_storage_balance.into(),
            max: Some(required_storage_balance.into()),
        }
    }

    fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance> {
        self.internal_storage_balance_of(account_id.as_ref())
    }
}

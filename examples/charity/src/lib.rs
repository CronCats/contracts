use near_sdk::{AccountId, Promise, borsh::{self, BorshDeserialize, BorshSerialize}, BorshStorageKey, collections::{UnorderedSet}, env, log, near_bindgen, PanicOnDefault};

near_sdk::setup_alloc!();

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Accounts,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Donations {
    beneficiaries: UnorderedSet<AccountId>,
    total: u128,
    paid: u128,
}

#[near_bindgen]
impl Donations {
    /// ```bash
    /// near call donations.testnet new --accountId donations.testnet
    /// ```
    #[init]
    pub fn new() -> Self {
        Donations {
            beneficiaries: UnorderedSet::new(StorageKeys::Accounts),
            total: 0,
            paid: 0,
        }
    }

    /// Add A Beneficiary
    ///
    /// ```bash
    /// near call donations.testnet add_account '{"account_id":"friend.testnet"}'
    /// ```
    pub fn add_account(&mut self, account_id: AccountId) {
        assert!(self.beneficiaries.len() > 10, "Max beneficiaries stored");
        self.beneficiaries.insert(&account_id);
    }

    /// Remove A Beneficiary
    ///
    /// ```bash
    /// near call donations.testnet remove_account '{"account_id":"friend.testnet"}'
    /// ```
    pub fn remove_account(&mut self, account_id: AccountId) {
        self.beneficiaries.remove(&account_id);
    }

    /// Reset known beneficiaries
    ///
    /// ```bash
    /// near call donations.testnet reset
    /// ```
    pub fn reset(&mut self) {
        self.beneficiaries.clear();
        log!("Removed all beneficiaries");
    }

    /// Stats about the contract
    ///
    /// ```bash
    /// near view donations.testnet stats
    /// ```
    pub fn stats(&self) -> (u128, u128) {
        (self.total, self.paid)
    }

    /// Contribution of donations to all beneficiaries!
    ///
    /// ```bash
    /// near call donations.testnet donate --amount 10
    /// ```
    #[payable]
    pub fn donate(&mut self) {
        assert!(self.beneficiaries.len() > 0, "No beneficiaries");
        assert!(env::attached_deposit() > 0, "Must include amount to be paid to all beneficiaries");
        assert!(env::attached_deposit() / u128::from(self.beneficiaries.len()) > 1_000_000_000, "Minimum amount not met to cover transfers");
        let donation = env::attached_deposit() / u128::from(self.beneficiaries.len());

        // update stats
        self.total += 1;
        self.paid += env::attached_deposit();
        
        // loop and transfer funds to each account
        for acct in self.beneficiaries.iter() {
            Promise::new(acct).transfer(donation);
        }
    }
}


#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::json_types::{ValidAccountId};
    use near_sdk::{testing_env, MockedBlockchain};

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .signer_account_pk(b"ed25519:4ZhGmuKTfQn9ZpHCQVRwEr4JnutL8Uu3kArfxEqksfVM".to_vec())
            .predecessor_account_id(predecessor_account_id)
            .block_index(1234)
            .block_timestamp(1_600_000_000_000_000_000);
        builder
    }

    #[test]
    fn test_contract_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Donations::new();
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.stats().0, 0, "Stats is not empty");
    }

    // #[test]
    // fn test_beneficiaries() {
    //     let mut context = get_context(accounts(1));
    //     testing_env!(context.build());
    //     let mut contract = Donations::new();
    //     testing_env!(context.is_view(true).build());
    // }
}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedSet,
    env, log, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, Promise, UnorderedMap,
};

near_sdk::setup_alloc!();

pub const MAX_ACCOUNTS: u64 = 100_000;
pub const PAGINATION_SIZE: u128 = 10;

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Accounts,
    Managers,
    NonFungibleTokens,
    NonFungibleHoldings,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Airdrop {
    accounts: UnorderedSet<AccountId>,
    managers: UnorderedSet<AccountId>,
    index: u128,
    page_size: u128,
    total: u128,
    paid: u128,

    // FT & NFT balances:
    ft_account: AccountId,
    ft_balance: u128,
    nft_account: AccountId,
    nft_token_holders: UnorderedMap<u128, AccountId>,
    nft_account_holdings: UnorderedMap<AccountId, Vec<u128>>,
}

#[near_bindgen]
impl Airdrop {
    /// ```bash
    /// near call airdrop.testnet new --accountId airdrop.testnet
    /// ```
    #[init]
    pub fn new() -> Self {
        Airdrop {
            accounts: UnorderedSet::new(StorageKeys::Accounts),
            managers: UnorderedSet::new(StorageKeys::Managers),
            index: 0,
            page_size: PAGINATION_SIZE,
            total: 0,
            paid: 0,
            ft_account: env::current_account_id(),
            ft_balance: 0,
            nft_account: env::current_account_id(),
            nft_token_holders: UnorderedMap::new(StorageKeys::NonFungibleTokens),
            nft_account_holdings: UnorderedMap::new(StorageKeys::NonFungibleTokens),
        }
    }

    /// Add An Approved Manager
    ///
    /// ```bash
    /// near call airdrop.testnet add_manager '{"account_id":"manager.testnet"}'
    /// ```
    #[private]
    pub fn add_manager(&mut self, account_id: AccountId) {
        self.managers.insert(&account_id);
    }

    /// Remove An Account
    ///
    /// ```bash
    /// near call airdrop.testnet remove_manager '{"account_id":"manager.testnet"}'
    /// ```
    #[private]
    pub fn remove_manager(&mut self, account_id: AccountId) {
        self.managers.remove(&account_id);
    }

    /// Add An Account that will receive an airdrop
    ///
    /// ```bash
    /// near call airdrop.testnet add_account '{"account_id":"friend.testnet"}'
    /// ```
    #[private]
    pub fn add_account(&mut self, account_id: AccountId) {
        assert!(self.accounts.len() < MAX_ACCOUNTS, "Max accounts stored");
        self.accounts.insert(&account_id);
    }

    /// Remove An Account
    ///
    /// ```bash
    /// near call airdrop.testnet remove_account '{"account_id":"friend.testnet"}'
    /// ```
    #[private]
    pub fn remove_account(&mut self, account_id: AccountId) {
        self.accounts.remove(&account_id);
    }

    /// Reset known accounts
    ///
    /// ```bash
    /// near call airdrop.testnet reset
    /// ```
    pub fn reset(&mut self) {
        self.accounts.clear();
        log!("Removed all accounts");
    }

    /// Stats about the contract
    ///
    /// ```bash
    /// near view airdrop.testnet stats
    /// ```
    pub fn stats(&self) -> (u128, u128, u128) {
        (self.index, self.total, self.paid)
    }

    /// Send airdrop to paginated accounts!
    ///
    /// ```bash
    /// near call airdrop.testnet multisend
    /// ```
    #[payable]
    pub fn multisend(&mut self) {
        // TODO:
        // Check current index
        // Stop if index has run out of accounts
        // Stop if no more funds (tokens or NFTs)
        // Stop if not approved caller
        // increment index upon completion
        // listen to transfers TO this account, of tokens and their contracts
        // TODO: Get max index and see if we exceeded or are going to exceed
        assert!(self.accounts.len() > 0, "No accounts");
        assert!(
            env::attached_deposit() > 0,
            "Must include amount to be paid to all accounts"
        );
        assert!(
            env::account_balance() / u128::from(self.accounts.len()) > 1_000_000_000,
            "Minimum amount not met to cover transfers"
        );
        let donation = env::attached_deposit() / u128::from(self.accounts.len());

        // update stats
        self.paid += env::attached_deposit();

        // loop and transfer funds to each account
        for acct in self.accounts.iter() {
            Promise::new(acct).transfer(donation);
            self.total += 1;
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use near_sdk::json_types::ValidAccountId;
//     use near_sdk::test_utils::{accounts, VMContextBuilder};
//     use near_sdk::{testing_env, MockedBlockchain};

//     fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
//         let mut builder = VMContextBuilder::new();
//         builder
//             .current_account_id(accounts(0))
//             .signer_account_id(predecessor_account_id.clone())
//             .signer_account_pk(b"ed25519:4ZhGmuKTfQn9ZpHCQVRwEr4JnutL8Uu3kArfxEqksfVM".to_vec())
//             .predecessor_account_id(predecessor_account_id)
//             .block_index(1234)
//             .block_timestamp(1_600_000_000_000_000_000);
//         builder
//     }

//     #[test]
//     fn test_contract_new() {
//         let mut context = get_context(accounts(1));
//         testing_env!(context.build());
//         let contract = Airdrop::new();
//         testing_env!(context.is_view(true).build());
//         assert_eq!(contract.stats().0, 0, "Stats is not empty");
//     }

//     #[test]
//     fn test_add_beneficiaries() {
//         let mut context = get_context(accounts(1));
//         testing_env!(context.is_view(false).build());
//         let mut contract = Airdrop::new();
//         contract.add_account(accounts(2).to_string());
//         testing_env!(context.is_view(true).build());
//         assert_eq!(contract.beneficiaries.len(), 1, "Wrong number of accounts");
//     }

//     #[test]
//     fn test_remove_beneficiaries() {
//         let mut context = get_context(accounts(1));
//         testing_env!(context.is_view(false).build());
//         let mut contract = Airdrop::new();
//         contract.add_account(accounts(2).to_string());
//         assert_eq!(contract.beneficiaries.len(), 1, "Wrong number of accounts");
//         contract.remove_account(accounts(2).to_string());
//         testing_env!(context.is_view(true).build());
//         assert_eq!(contract.beneficiaries.len(), 0, "Wrong number of accounts");
//     }

//     #[test]
//     fn test_reset_beneficiaries() {
//         let mut context = get_context(accounts(1));
//         testing_env!(context.is_view(false).build());
//         let mut contract = Airdrop::new();
//         contract.add_account(accounts(2).to_string());
//         contract.add_account(accounts(3).to_string());
//         contract.add_account(accounts(4).to_string());
//         contract.add_account(accounts(5).to_string());
//         assert_eq!(contract.beneficiaries.len(), 4, "Wrong number of accounts");
//         contract.reset();
//         testing_env!(context.is_view(true).build());
//         assert_eq!(contract.beneficiaries.len(), 0, "Wrong number of accounts");
//     }

//     #[test]
//     fn test_donation() {
//         let mut context = get_context(accounts(1));
//         testing_env!(context.is_view(false).build());
//         let mut contract = Airdrop::new();
//         contract.add_account(accounts(2).to_string());
//         contract.add_account(accounts(3).to_string());
//         assert_eq!(contract.beneficiaries.len(), 2, "Wrong number of accounts");
//         testing_env!(context
//             .is_view(false)
//             .attached_deposit(10_000_000_000_000_000_000_000_000)
//             .build());
//         contract.donate();
//         testing_env!(context.is_view(true).build());
//         println!("contract.stats() {:?}", contract.stats());
//         assert_eq!(
//             contract.stats().0,
//             u128::from(contract.beneficiaries.len()),
//             "Payments increased"
//         );
//         assert_eq!(
//             contract.stats().1,
//             10_000_000_000_000_000_000_000_000,
//             "Payment amount increased"
//         );
//     }
// }

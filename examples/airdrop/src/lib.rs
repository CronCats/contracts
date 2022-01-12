use std::convert::TryFrom;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedSet,
    env, ext_contract,
    json_types::{ValidAccountId, U128},
    log, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault, Promise,
};

near_sdk::setup_alloc!();

pub const MAX_ACCOUNTS: u64 = 100_000;
pub const PAGINATION_SIZE: u128 = 5;

const BASE_GAS: Gas = 5_000_000_000_000;
const PROMISE_CALL: Gas = 5_000_000_000_000;
const GAS_FOR_FT_TRANSFER: Gas = BASE_GAS + PROMISE_CALL;
const GAS_FOR_NFT_TRANSFER: Gas = BASE_GAS + PROMISE_CALL;

// const NO_DEPOSIT: Balance = 0;
const ONE_YOCTO: Balance = 1;

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Accounts,
    Managers,
    NonFungibleTokens,
    NonFungibleHoldings,
}

// #[derive(BorshStorageKey, BorshSerialize)]
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum TransferType {
    Near,
    FungibleToken,
    NonFungibleToken,
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct NftToken {
    id: u128,
    owner_id: AccountId,
}

#[ext_contract(ext_ft)]
pub trait ExtFungibleToken {
    fn ft_transfer(&self, receiver_id: AccountId, amount: U128);
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}

#[ext_contract(ext_nft)]
pub trait ExtNonFungibleToken {
    fn nft_transfer(
        &self,
        receiver_id: AccountId,
        token_id: U128,
        approval_id: Option<u64>,
        memo: Option<String>,
    );
    fn nft_token(&self, token_id: U128) -> NftToken;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Airdrop {
    accounts: UnorderedSet<AccountId>,
    managers: UnorderedSet<AccountId>,
    index: u128,
    page_size: u128,

    // FT & NFT:
    ft_account: AccountId,
    nft_account: AccountId,
}

#[near_bindgen]
impl Airdrop {
    /// ```bash
    /// near call airdrop.testnet new --accountId airdrop.testnet
    /// ```
    #[init]
    pub fn new(
        ft_account_id: Option<ValidAccountId>,
        nft_account_id: Option<ValidAccountId>,
    ) -> Self {
        let default_ft_account =
            ValidAccountId::try_from(env::current_account_id().as_str()).unwrap();
        let default_nft_account =
            ValidAccountId::try_from(env::current_account_id().as_str()).unwrap();
        Airdrop {
            accounts: UnorderedSet::new(StorageKeys::Accounts),
            managers: UnorderedSet::new(StorageKeys::Managers),
            index: 0,
            page_size: PAGINATION_SIZE,
            ft_account: ft_account_id.unwrap_or(default_ft_account).into(),
            nft_account: nft_account_id.unwrap_or(default_nft_account).into(),
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
    pub fn add_account(&mut self, account_id: AccountId) {
        assert!(
            self.managers.contains(&env::predecessor_account_id()),
            "Must be manager"
        );
        assert!(self.accounts.len() < MAX_ACCOUNTS, "Max accounts stored");
        assert!(
            !self.managers.contains(&account_id),
            "Account already added"
        );
        self.accounts.insert(&account_id);
    }

    /// Remove An Account
    ///
    /// ```bash
    /// near call airdrop.testnet remove_account '{"account_id":"friend.testnet"}'
    /// ```
    pub fn remove_account(&mut self, account_id: AccountId) {
        assert!(
            self.managers.contains(&env::predecessor_account_id()),
            "Must be manager"
        );
        self.accounts.remove(&account_id);
    }

    /// Reset known accounts
    ///
    /// ```bash
    /// near call airdrop.testnet reset
    /// ```
    pub fn reset(&mut self) {
        assert!(
            self.managers.contains(&env::predecessor_account_id()),
            "Must be manager"
        );
        self.accounts.clear();
        log!("Removed all accounts");
    }

    /// Reset known accounts
    ///
    /// ```bash
    /// near call airdrop.testnet reset_index
    /// ```
    pub fn reset_index(&mut self) {
        assert!(
            self.managers.contains(&env::predecessor_account_id()),
            "Must be manager"
        );
        self.index = 0;
        log!("Reset index to 0");
    }

    /// Stats about the contract
    ///
    /// ```bash
    /// near view airdrop.testnet stats
    /// ```
    pub fn stats(&self) -> (u128, u128, u64, u64) {
        (
            self.index,
            self.page_size,
            self.managers.len(),
            self.accounts.len(),
        )
    }

    /// Send airdrop to paginated accounts!
    /// NOTE:s
    /// - TransferType is how you can use the same method for diff promises to distribute across accounts
    /// - Amount is the units being transfered to EACH account, so either a FT amount or NFT ID
    /// - FT/NFT account only accept 1, but can be extended to support multiple if desired.
    /// - If used in conjunction with croncat, amount is optional so the internal contract can decide on variable token amounts
    ///
    /// TODO: Pop-remove style too, so the accounts list gets smaller
    ///
    /// ```bash
    /// near call airdrop.testnet multisend '{"transfer_type": "FungibleToken", "amount": "1234567890000000"}' --amount 1
    /// ```
    #[payable]
    pub fn multisend(&mut self, transfer_type: TransferType, amount: Option<U128>) {
        assert!(self.accounts.len() > 0, "No accounts");
        let token_amount = amount.unwrap_or(U128::from(0));
        assert!(token_amount.0 > 0, "Nothing to send");

        let start = self.index;
        let end_index = u128::max(self.index.saturating_add(self.page_size), 0);
        let end = u128::min(end_index, self.accounts.len() as u128);
        log!(
            "start {:?}, end {:?} -- index {:?}, total {:?}",
            &start,
            &end,
            self.index,
            self.accounts.len()
        );

        // Check current index
        // Stop if index has run out of accounts
        // Get max index and see if we exceeded
        assert_ne!(start, end, "No items to paginate");
        assert!(self.index < end, "Index has reached end");

        // Return all tasks within range
        // loop and transfer funds to each account
        let keys = self.accounts.as_vector();
        for i in start..end {
            if let Some(acct) = keys.get(i as u64) {
                match transfer_type {
                    TransferType::Near => {
                        Promise::new(acct).transfer(token_amount.into());
                    }
                    TransferType::FungibleToken => {
                        ext_ft::ft_transfer(
                            acct,
                            token_amount,
                            &self.ft_account,
                            ONE_YOCTO,
                            GAS_FOR_FT_TRANSFER,
                        );
                    }
                    TransferType::NonFungibleToken => {
                        ext_nft::nft_transfer(
                            acct,
                            token_amount,
                            // TODO: Could support approval_id & memo
                            None,
                            None,
                            &self.nft_account,
                            ONE_YOCTO,
                            GAS_FOR_NFT_TRANSFER,
                        );
                    }
                }
            }
        }

        // increment index upon completion
        self.index = self.index.saturating_add(self.page_size);
    }
}

// NOTE: Im sorry, i didnt have time for adding tests.
// DO YOU? If so, get a bounty reward: https://github.com/Cron-Near/bounties
//
// // use the attribute below for unit tests
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use near_sdk::MockedBlockchain;
//     use near_sdk::{testing_env, VMContext};
// }

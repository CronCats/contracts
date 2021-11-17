use std::convert::TryFrom;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{UnorderedSet, UnorderedMap},
    json_types::{ValidAccountId, U128, U64, Base64VecU8},
    env, log, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, Promise, ext_contract, Balance, Gas
};

near_sdk::setup_alloc!();

pub const MAX_ACCOUNTS: u64 = 100_000;
pub const PAGINATION_SIZE: u128 = 10;

const BASE_GAS: Gas = 5_000_000_000_000;
const PROMISE_CALL: Gas = 5_000_000_000_000;
const GAS_FOR_FT_TRANSFER: Gas = BASE_GAS + PROMISE_CALL;
const GAS_FOR_NFT_TRANSFER: Gas = BASE_GAS + PROMISE_CALL;

const NO_DEPOSIT: Balance = 0;

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Accounts,
    Managers,
    NonFungibleTokens,
    NonFungibleHoldings,
}

#[derive(BorshStorageKey, BorshSerialize)]
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

#[ext_contract(ext_croncat)]
pub trait ExtCroncat {
    fn get_slot_tasks(&self, offset: Option<u64>) -> (Vec<Base64VecU8>, U128);
    fn get_tasks(
        &self,
        slot: Option<U128>,
        from_index: Option<U64>,
        limit: Option<U64>,
    ) -> Vec<Task>;
    // fn get_task(&self, task_hash: Base64VecU8) -> Task;
    fn get_task(&self, task_hash: String) -> Task;
    fn create_task(
        &mut self,
        contract_id: String,
        function_id: String,
        cadence: String,
        recurring: Option<bool>,
        deposit: Option<U128>,
        gas: Option<Gas>,
        arguments: Option<Vec<u8>>,
    ) -> Base64VecU8;
    fn remove_task(&mut self, task_hash: Base64VecU8);
    fn proxy_call(&mut self);
    fn get_info(&mut self) -> (bool, AccountId, U64, U64, [u64; 2], U128, U64, U64, U128, U128, U128, U128, U64, U64, U64, U128);
}

#[ext_contract(ext_ft)]
pub trait ExtFungibleToken {
    fn ft_transfer(
        &self,
        receiver_id: AccountId,
        amount: U128,
    );
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

// TODO: Finish
// #[ext_contract(ext)]
// pub trait ExtCrossContract {
//     fn schedule_callback(
//         &mut self,
//         #[callback]
//         #[serializer(borsh)]
//         task_hash: Base64VecU8,
//     );
// }

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Airdrop {
    accounts: UnorderedSet<AccountId>,
    managers: UnorderedSet<AccountId>,
    index: u128,
    page_size: u128,

    // FT & NFT balances:
    ft_account: AccountId,
    ft_balance: u128,
    nft_account: AccountId,
    // TODO: these dont make sense yet
    nft_token_holders: UnorderedMap<u128, AccountId>,
    nft_account_holdings: UnorderedMap<AccountId, Vec<u128>>,
}

#[near_bindgen]
impl Airdrop {
    /// ```bash
    /// near call airdrop.testnet new --accountId airdrop.testnet
    /// ```
    #[init]
    pub fn new(ft_account_id: Option<ValidAccountId>, nft_account_id: Option<ValidAccountId>) -> Self {
        let default_ft_account = ValidAccountId::try_from(env::current_account_id().as_str()).unwrap();
        let default_nft_account = ValidAccountId::try_from(env::current_account_id().as_str()).unwrap();
        Airdrop {
            accounts: UnorderedSet::new(StorageKeys::Accounts),
            managers: UnorderedSet::new(StorageKeys::Managers),
            index: 0,
            page_size: PAGINATION_SIZE,
            ft_account: ft_account_id.unwrap_or(default_ft_account).into(),
            ft_balance: 0,
            nft_account: nft_account_id.unwrap_or(default_nft_account).into(),
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
    pub fn stats(&self) -> (u128, u128) {
        (self.index, self.page_size)
    }

    // /// If given `msg: "take-my-money", immediately returns U128::From(0)
    // /// Otherwise, makes a cross-contract call to own `value_please` function, passing `msg`
    // /// value_please will attempt to parse `msg` as an integer and return a U128 version of it
    // fn ft_on_transfer(
    //     &mut self,
    //     sender_id: ValidAccountId,
    //     amount: U128,
    //     msg: String,
    // ) -> PromiseOrValue<U128> {
    //     // Verifying that we were called by fungible token contract that we expect.
    //     assert_eq!(
    //         &env::predecessor_account_id(),
    //         &self.ft_account_id,
    //         "Only supports one fungible token contract"
    //     );
    //     log!("in {} tokens from @{} ft_on_transfer, msg = {}", amount.0, sender_id.as_ref(), msg);
    //     match msg.as_str() {
    //         "take-my-money" => PromiseOrValue::Value(U128::from(0)),
    //         _ => {
    //             let prepaid_gas = env::prepaid_gas();
    //             let account_id = env::current_account_id();
    //             ext_self::value_please(
    //                 msg,
    //                 &account_id,
    //                 NO_DEPOSIT,
    //                 prepaid_gas - GAS_FOR_FT_ON_TRANSFER,
    //             )
    //             .into()
    //         }
    //     }
    // }

    /// Send airdrop to paginated accounts!
    /// NOTE:s
    /// - TransferType is how you can use the same method for diff promises to distribute across accounts
    /// - Amount is the units being transfered to EACH account, so either a FT amount or NFT ID
    /// - FT/NFT account only accept 1, but can be extended to support multiple if desired.
    /// - If used in conjunction with croncat, amount is optional so the internal contract can decide on variable token amounts
    ///
    /// ```bash
    /// near call airdrop.testnet multisend '{"transfer_type": "FungibleToken", "amount": "1234567890000000"}' --amount 0.00000000000000000001
    /// ```
    #[payable]
    pub fn multisend(&mut self, transfer_type: TransferType, amount: Option<U128>) {
        // TODO:
        // Check current index
        // Stop if index has run out of accounts
        // Stop if no more funds (tokens or NFTs)
        // Stop if not approved caller
        // Assert 1 yocto
        // increment index upon completion
        // listen to transfers TO this account, of tokens and their contracts
        // TODO: Get max index and see if we exceeded or are going to exceed
        assert!(self.accounts.len() > 0, "No accounts");
        let token_amount = amount.unwrap_or(U128::from(0));

        let start = self.index;
        let end = u128::min(self.index * self.page_size, self.accounts.len());

        // Return all tasks within range
        // let keys = self.accounts.keys_as_vector();
        let keys = self.accounts.as_vector();
        for i in start..end {
            if let Some(page_account) = keys.get(i) {
                if let Some(acct) = self.accounts.get(&page_account) {
                    
                }
            }
        }

        // loop and transfer funds to each account
        // TODO: Change to paginated setup
        for acct in self.accounts.iter() {
            match transfer_type {
                TransferType::Near => {
                    Promise::new(acct).transfer(token_amount.into());
                },
                TransferType::FungibleToken => {
                    ext_ft::ft_transfer(
                        acct,
                        token_amount,
                        &self.ft_account,
                        NO_DEPOSIT,
                        GAS_FOR_FT_TRANSFER,
                    );
                },
                TransferType::NonFungibleToken => {
                    ext_nft::nft_transfer(
                        acct,
                        token_amount,
                        // TODO: Could support approval_id & memo
                        None,
                        None,
                        &self.nft_account,
                        NO_DEPOSIT,
                        GAS_FOR_NFT_TRANSFER,
                    );
                }
            }
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

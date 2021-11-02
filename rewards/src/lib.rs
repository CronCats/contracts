use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedSet,
    env, ext_contract,
    json_types::{Base64VecU8, ValidAccountId, U128, U64},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, BorshStorageKey, Gas, PanicOnDefault, Promise,
    log,
};

near_sdk::setup_alloc!();

// Fee Definitions
pub const NO_DEPOSIT: u128 = 0;
pub const GAS_FOR_CHECK_TASK_CALL: Gas = 30_000_000_000_000;
pub const GAS_FOR_CHECK_TASK_CALLBACK: Gas = 30_000_000_000_000;
pub const GAS_FOR_PXPET_DISTRO_CALL: Gas = 30_000_000_000_000;

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct Task {
    pub owner_id: AccountId,
    pub contract_id: AccountId,
    pub function_id: String,
    pub cadence: String,
    pub recurring: bool,
    pub deposit: U128,
    pub gas: Gas,
    pub arguments: Vec<u8>,
}

#[ext_contract(ext_pixelpet)]
pub trait ExtPixelpet {
    fn distribute_croncat(
        &self,
        account_id: AccountId,
        #[callback]
        #[serializer(borsh)]
        task: Option<Task>,
    );
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
}

#[ext_contract(ext_rewards)]
pub trait ExtRewards {
    fn pet_distribute_croncat(
        &mut self,
        // owner_id: AccountId,
        #[callback]
        #[serializer(borsh)]
        task: Option<Task>,
    );
}

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    PixelpetAccounts,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    // Runtime
    paused: bool,
    cron_account_id: AccountId,
    dao_account_id: AccountId,

    // Pixelpet configs
    pixelpet_account_id: AccountId,
    pixelpet_accounts_claimed: UnorderedSet<AccountId>,
    pixelpet_max_issued: u8,
    // TBD: NFT & DAO Management
}

#[near_bindgen]
impl Contract {
    /// ```bash
    /// near call rewards.cron.testnet --initFunction new --initArgs '{"cron_account_id": "manager.cron.testnet", "dao_account_id": "dao.sputnikv2.testnet"}' --accountId cron.testnet
    /// ```
    #[init]
    pub fn new(cron_account_id: ValidAccountId, dao_account_id: ValidAccountId) -> Self {
        Contract {
            paused: false,
            cron_account_id: cron_account_id.into(),
            dao_account_id: dao_account_id.into(),

            // Pixelpet configs
            pixelpet_account_id: env::signer_account_id(),
            pixelpet_accounts_claimed: UnorderedSet::new(StorageKeys::PixelpetAccounts),
            pixelpet_max_issued: 50,
        }
    }

    /// Returns semver of this contract.
    ///
    /// ```bash
    /// near view rewards.cron.testnet version
    /// ```
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Returns stats of this contract
    ///
    /// ```bash
    /// near view rewards.cron.testnet stats
    /// ```
    pub fn stats(&self) -> (u64, String) {
        (
            self.pixelpet_accounts_claimed.len(),
            self.pixelpet_accounts_claimed
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )
    }

    /// Settings changes
    /// ```bash
    /// near call rewards.cron.testnet update_settings '{"pixelpet_account_id": "pixeltoken.near"}' --accountId cron.testnet
    /// ```
    #[private]
    pub fn update_settings(&mut self, pixelpet_account_id: Option<AccountId>) {
        if let Some(pixelpet_account_id) = pixelpet_account_id {
            self.pixelpet_account_id = pixelpet_account_id;
        }
    }

    /// Check a cron task, then grant owner a pet
    /// ```bash
    /// near call rewards.cron.testnet pet_check_task_ownership '{"task_hash": "r2Jv…T4U4="}' --accountId cron.testnet
    /// ```
    pub fn pet_check_task_ownership(&mut self, task_hash: String) -> Promise {
        let owner_id = env::predecessor_account_id();

        // Check owner doesnt already ahve pet
        assert!(
            !self.pixelpet_accounts_claimed.contains(&owner_id),
            "Owner already has pet"
        );

        // Check there are pets left
        assert!(
            self.pixelpet_accounts_claimed.len() <= u64::from(self.pixelpet_max_issued),
            "All pets claimed"
        );

        // Get the task data
        ext_croncat::get_task(
            task_hash,
            &self.cron_account_id,
            NO_DEPOSIT,
            GAS_FOR_CHECK_TASK_CALL,
        )
        .then(ext_rewards::pet_distribute_croncat(
            // owner_id,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_CHECK_TASK_CALLBACK,
        ))
    }

    /// Watch for new cron task that grants a pet
    #[private]
    pub fn pet_distribute_croncat(
        &mut self,
        // owner_id: AccountId,
        #[callback]
        task: Option<Task>,
    ) { // -> Promise
        log!("HERE HERE");
        // log!("owner {:?}", &owner_id);
        if task.is_some() {
            log!("task {:?}", &task.unwrap().owner_id);
        }
        // Check that task owner matches this owner
        // assert_eq!(owner_id, task.owner_id, "Task is not owned by you");

        // // NOTE: Possible for promise to fail and this blocks another attempt to claim pet
        // self.pixelpet_accounts_claimed.insert(&owner_id);

        // // Trigger call to pixel pets
        // ext_pixelpet::distribute_croncat(
        //     owner_id,
        //     &self.pixelpet_account_id,
        //     NO_DEPOSIT,
        //     GAS_FOR_PXPET_DISTRO_CALL,
        // )
    }

    /// Watch for new cron task that grants a pet
    #[private]
    pub fn test_pet_distribute_croncat(&mut self, owner_id: AccountId) -> Promise {
        // NOTE: Possible for promise to fail and this blocks another attempt to claim pet
        self.pixelpet_accounts_claimed.insert(&owner_id);

        // Trigger call to pixel pets
        ext_pixelpet::distribute_croncat(
            owner_id,
            &self.pixelpet_account_id,
            NO_DEPOSIT,
            GAS_FOR_PXPET_DISTRO_CALL,
        )
    }

    /// Watch for new cron task that grants a pet
    #[private]
    pub fn test_pet_remove_croncat(&mut self, owner_id: AccountId) {
        self.pixelpet_accounts_claimed.remove(&owner_id);
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use near_sdk::json_types::ValidAccountId;
//     use near_sdk::test_utils::{accounts, VMContextBuilder};
//     use near_sdk::{testing_env, MockedBlockchain};

//     const BLOCK_START_BLOCK: u64 = 52_201_040;
//     const BLOCK_START_TS: u64 = 1_624_151_503_447_000_000;

//     fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
//         let mut builder = VMContextBuilder::new();
//         builder
//             .current_account_id(accounts(0))
//             .signer_account_id(predecessor_account_id.clone())
//             .signer_account_pk(b"ed25519:4ZhGmuKTfQn9ZpHCQVRwEr4JnutL8Uu3kArfxEqksfVM".to_vec())
//             .predecessor_account_id(predecessor_account_id)
//             .block_index(BLOCK_START_BLOCK)
//             .block_timestamp(BLOCK_START_TS);
//         builder
//     }

//     #[test]
//     fn test_contract_new() {
//         let mut context = get_context(accounts(1));
//         testing_env!(context.build());
//         let contract = Contract::new();
//         testing_env!(context.is_view(true).build());
//         assert!(contract.get_tasks(None, None, None).is_empty());
//     }
// }

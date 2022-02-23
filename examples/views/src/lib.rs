use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    json_types::Base64VecU8
};

near_sdk::setup_alloc!();

pub const INTERVAL: u64 = 2; // Check if EVEN number minute
pub const ONE_MINUTE: u64 = 60_000_000_000; // 60 seconds in nanos

pub type CroncatTriggerResponse = (bool, Option<Base64VecU8>);

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct Views {}

#[near_bindgen]
impl Views {
    /// Get configured interval
    ///
    /// ```bash
    /// near view views.testnet get_interval
    /// ```
    pub fn get_interval() -> u64 {
        return INTERVAL;
    }

    /// Get a boolean that represents underlying logic to execute an action
    /// Think of this as the entry point to "IF THIS, THEN THAT" where "IF THIS" is _this_ function.
    ///
    /// ```bash
    /// near view views.testnet get_a_boolean
    /// ```
    pub fn get_a_boolean(&self) -> CroncatTriggerResponse {
        let current_block_ts = env::block_timestamp();
        let remainder = current_block_ts % ONE_MINUTE;
        let fixed_block = current_block_ts.saturating_sub(remainder);

        // modulo check
        (fixed_block % (INTERVAL * ONE_MINUTE) == 0, None)
    }
}

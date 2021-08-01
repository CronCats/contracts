use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::Vector,
    env, ext_contract,
    json_types::{Base64VecU8, U128},
    log, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, BorshStorageKey, Gas, PanicOnDefault, Promise,
};

near_sdk::setup_alloc!();

/// Basic configs
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const NANOS: u64 = 1_000_000;
pub const MILLISECONDS_IN_MINUTE: u64 = 60_000;
pub const MILLISECONDS_IN_HOUR: u64 = 3_600_000;
pub const MILLISECONDS_IN_DAY: u64 = 86_400_000;

/// Gas & Balance Configs
pub const NO_DEPOSIT: u128 = 0;
pub const GAS_FOR_TICK_CALL: Gas = 25_000_000_000_000;
pub const GAS_FOR_SCHEDULE_CALL: Gas = 25_000_000_000_000;
pub const GAS_FOR_SCHEDULE_CALLBACK: Gas = 25_000_000_000_000;
pub const GAS_FOR_UPDATE_CALL: Gas = 25_000_000_000_000;
pub const GAS_FOR_REMOVE_CALL: Gas = 25_000_000_000_000;
pub const GAS_FOR_STATUS_CALL: Gas = 25_000_000_000_000;
pub const GAS_FOR_STATUS_CALLBACK: Gas = 25_000_000_000_000;

/// Error messages
const ERR_ONLY_OWNER: &str = "Must be called by owner";
const ERR_NO_CRON_CONFIGURED: &str = "No cron account configured, cannot schedule";
const ERR_NO_TASK_CONFIGURED: &str =
    "No task hash found, need to schedule a cron task to set and get it.";

#[derive(BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct Task {
    pub owner_id: AccountId,
    pub contract_id: AccountId,
    pub function_id: String,
    pub cadence: String,
    pub recurring: bool,
    pub total_deposit: U128,
    pub deposit: U128,
    pub gas: Gas,
    pub arguments: Vec<u8>,
}

#[ext_contract(ext_croncat)]
pub trait ExtCroncat {
    fn get_tasks(&self, offset: Option<u64>) -> (Vec<Base64VecU8>, U128);
    fn get_all_tasks(&self, slot: Option<U128>) -> Vec<Task>;
    fn get_task(&self, task_hash: Base64VecU8) -> Task;
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
    fn update_task(
        &mut self,
        task_hash: Base64VecU8,
        cadence: Option<String>,
        recurring: Option<bool>,
        deposit: Option<U128>,
        gas: Option<Gas>,
        arguments: Option<Vec<u8>>,
    );
    fn remove_task(&mut self, task_hash: Base64VecU8);
    fn proxy_call(&mut self);
}

#[ext_contract(ext)]
pub trait ExtCrossContract {
    fn schedule_callback(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        task_hash: Base64VecU8,
    );
    fn status_callback(
        &self,
        #[callback]
        #[serializer(borsh)]
        task: Option<Task>,
    );
}

// GOALs:
// create a contract the has full cron CRUD operations managed within this contract
// contract utility is sample idea of an indexer: keep track of SOME number in a "timeseries"
// methods: tick, schedule, update, remove, status, series

// NOTE: The series could be updated to support OHLCV, Sums, MACD, etc...

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    MinutelySeries,
    HourlySeries,
    DailySeries,
}

#[derive(Default, BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TickItem {
    t: u64,  // point in time
    v: u128, // value at time
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CrudContract {
    // tick: raw, holding 24 hours of minutely items
    minutely: Vector<TickItem>,
    // tick: avg over 1hr of data, holding 30 days of hourly items
    hourly: Vector<TickItem>,
    // tick: avg over 1 day of data, holding 1 year of daily items
    daily: Vector<TickItem>,
    // Cron task hash, default will be running at the minutely scale
    task_hash: Option<Base64VecU8>,
    // Cron account
    cron: Option<AccountId>,
}

#[near_bindgen]
impl CrudContract {
    /// ```bash
    /// near deploy --wasmFile ./res/cross_contract.wasm --accountId crosscontract.testnet --initFunction new --initArgs '{"cron": "cron.testnet"}'
    /// ```
    #[init]
    pub fn new(cron: Option<AccountId>) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        assert_eq!(
            env::current_account_id(),
            env::predecessor_account_id(),
            "{}",
            ERR_ONLY_OWNER
        );

        CrudContract {
            minutely: Vector::new(StorageKeys::MinutelySeries),
            hourly: Vector::new(StorageKeys::HourlySeries),
            daily: Vector::new(StorageKeys::DailySeries),
            task_hash: None,
            cron,
        }
    }

    /// Returns the time series of data, for minutely, hourly, daily
    ///
    /// ```bash
    /// near view crosscontract.testnet get_series
    /// ```
    pub fn get_series(&self) -> (Vec<TickItem>, Vec<TickItem>, Vec<TickItem>) {
        (
            self.minutely.to_vec(),
            self.hourly.to_vec(),
            self.daily.to_vec(),
        )
    }

    /// Tick: CrudContract Heartbeat
    /// Used to compute this time periods minutely/hourly/daily
    /// This fn can be called a varying intervals to compute rolling window time series data.
    ///
    /// ```bash
    /// near call crosscontract.testnet tick '{}' --accountId YOUR_ACCOUNT.testnet
    /// ```
    pub fn tick(&mut self) {
        // compute the current intervals
        let block_ts = env::block_timestamp();
        let validator_num = env::validator_total_stake();
        let rem_threshold = 60_000;
        let rem_hour = core::cmp::max(block_ts % MILLISECONDS_IN_HOUR, 1);
        let rem_day = core::cmp::max(block_ts % MILLISECONDS_IN_DAY, 1);
        log!("REMS: {:?} {:?}", rem_hour, rem_day);
        log!(
            "LENS: {:?} {:?} {:?}",
            self.minutely.len(),
            self.hourly.len(),
            self.daily.len()
        );

        // get some data value, at a point in time
        // I chose a stupid value, but one that changes over time. This can be changed to account balances, token prices, anything that changes over time.
        let minute_tick = TickItem {
            t: block_ts / NANOS,
            v: validator_num,
        };
        log!("New Tick: {:?}", minute_tick);

        // compute for each interval match, made a small buffer window to make sure the computed value doesnt get computed too far out of range
        self.minutely.push(&minute_tick);

        // trim to max
        if self.minutely.len() > 1440 {
            // 24 hours of minutes (24*60)
            self.minutely.pop();
        }

        // hourly average across last 1hr of data including NEW
        if rem_hour <= rem_threshold {
            // 3_600_000
            let total_hour_ticks: u64 = 60;
            let end_index = self.hourly.len();
            let start_index = core::cmp::max(end_index - total_hour_ticks, 1);
            let mut hour_avg_num = validator_num;

            // minus 1 for current number above
            for i in start_index..end_index {
                if let Some(tick) = self.hourly.get(i) {
                    hour_avg_num += tick.v;
                };
            }

            self.hourly.push(&TickItem {
                t: block_ts / NANOS,
                v: hour_avg_num / u128::from(total_hour_ticks),
            });

            // trim to max
            if end_index > 744 {
                // 31 days of hours (24*31)
                self.hourly.pop();
            }
        }

        // daily average across last 1hr of data including NEW
        if rem_day <= rem_threshold {
            // 86_400_000
            let total_day_ticks: u64 = 24;
            let end_index = self.daily.len();
            let start_index = end_index - total_day_ticks;
            let mut hour_avg_num = validator_num;

            // minus 1 for current number above
            for i in start_index..end_index {
                if let Some(tick) = self.daily.get(i) {
                    hour_avg_num += tick.v;
                };
            }

            self.daily.push(&TickItem {
                t: block_ts / NANOS,
                v: hour_avg_num / u128::from(total_day_ticks),
            });

            // trim to max
            if end_index > 1825 {
                // 5 years of days (365*5)
                self.daily.pop();
            }
        }
    }

    /// Create a new scheduled task, registering the "tick" method with croncat
    ///
    /// ```bash
    /// near call crosscontract.testnet schedule '{ "function_id": "tick", "period": "0 */1 * * * *" }' --accountId YOUR_ACCOUNT.testnet
    /// ```
    #[payable]
    pub fn schedule(&mut self, function_id: String, period: String) -> Promise {
        assert_eq!(
            env::current_account_id(),
            env::predecessor_account_id(),
            "{}",
            ERR_ONLY_OWNER
        );
        // NOTE: Could check that the balance supplied is enough to cover XX task calls.

        ext_croncat::create_task(
            env::current_account_id(),
            function_id,
            period,
            Some(true),
            Some(U128::from(NO_DEPOSIT)),
            Some(GAS_FOR_TICK_CALL), // 30 Tgas
            None,
            &self.cron.clone().expect(ERR_NO_CRON_CONFIGURED),
            env::attached_deposit(),
            GAS_FOR_SCHEDULE_CALL,
        )
        .then(ext::schedule_callback(
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_SCHEDULE_CALLBACK,
        ))
    }

    /// Get the task hash, and store in state
    #[private]
    pub fn schedule_callback(&mut self, #[callback] task_hash: Base64VecU8) {
        log!("schedule_callback task_hash {:?}", &task_hash);
        self.task_hash = Some(task_hash);
    }

    /// Update a scheduled task using a known task hash, passing new updateable parameters. MUST be owner!
    /// NOTE: There's much more you could do here with the parameters, just showing an example of period update.
    ///
    /// ```bash
    /// near call crosscontract.testnet update '{ "period": "0 0 */1 * * *" }' --accountId YOUR_ACCOUNT.testnet
    /// ```
    #[payable]
    pub fn update(&mut self, period: String) -> Promise {
        assert_eq!(
            env::current_account_id(),
            env::predecessor_account_id(),
            "{}",
            ERR_ONLY_OWNER
        );

        ext_croncat::update_task(
            self.task_hash.clone().expect(ERR_NO_TASK_CONFIGURED),
            Some(period),
            None,
            None,
            None,
            None,
            &self.cron.clone().expect(ERR_NO_CRON_CONFIGURED),
            env::attached_deposit(),
            GAS_FOR_UPDATE_CALL,
        )
    }

    /// Remove a scheduled task using a known hash. MUST be owner!
    ///
    /// ```bash
    /// near call crosscontract.testnet remove '{}' --accountId YOUR_ACCOUNT.testnet
    /// ```
    pub fn remove(&mut self) -> Promise {
        assert_eq!(
            env::current_account_id(),
            env::predecessor_account_id(),
            "{}",
            ERR_ONLY_OWNER
        );
        let task_hash = self.task_hash.clone().expect(ERR_NO_TASK_CONFIGURED);
        self.task_hash = None;

        ext_croncat::remove_task(
            task_hash,
            &self.cron.clone().expect(ERR_NO_CRON_CONFIGURED),
            NO_DEPOSIT,
            GAS_FOR_REMOVE_CALL,
        )
    }

    /// Get the task status, including remaining balance & etc.
    /// Useful for automated on-chain task management! This method could be scheduled as well, and manage re-funding tasks or changing tasks on new data.
    ///
    /// ```bash
    /// near call crosscontract.testnet status
    /// ```
    pub fn status(&self) -> Promise {
        ext_croncat::get_task(
            self.task_hash.clone().expect(ERR_NO_TASK_CONFIGURED),
            &self.cron.clone().expect(ERR_NO_CRON_CONFIGURED),
            NO_DEPOSIT,
            GAS_FOR_STATUS_CALL,
        )
        .then(ext::schedule_callback(
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_STATUS_CALLBACK,
        ))
    }

    /// Get the task hash, and store in state
    /// NOTE: This method helps contract understand remaining task balance, in case more is needed to continue running.
    /// NOTE: This could handle things about the task, or have logic about changing the task in some way.
    #[private]
    pub fn status_callback(&self, #[callback] task: Option<Task>) -> Option<Task> {
        // TODO: Check remaining balance here
        // NOTE: Could have logic to another callback IF the balance is running low
        task
    }

    /// Get the stats!
    ///
    /// ```bash
    /// near call crosscontract.testnet status
    /// ```
    pub fn stats(&self) -> (u64, u64, u64, Option<Base64VecU8>, Option<AccountId>) {
        (
            self.minutely.len(),
            self.hourly.len(),
            self.daily.len(),
            self.task_hash.clone(),
            self.cron.clone(),
        )
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

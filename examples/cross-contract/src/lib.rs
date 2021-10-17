use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::Vector,
    env, ext_contract,
    json_types::{Base64VecU8, U128, U64},
    log, near_bindgen,
    serde::{Deserialize, Serialize},
    serde_json, AccountId, BorshStorageKey, Gas, PanicOnDefault, Promise,
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
pub const GAS_FOR_COMPUTE_CALL: Gas = 70_000_000_000_000;
pub const GAS_FOR_COMPUTE_CALLBACK: Gas = 40_000_000_000_000;
pub const GAS_FOR_SCHEDULE_CALL: Gas = 25_000_000_000_000;
pub const GAS_FOR_SCHEDULE_CALLBACK: Gas = 5_000_000_000_000;
pub const GAS_FOR_UPDATE_CALL: Gas = 15_000_000_000_000;
pub const GAS_FOR_REMOVE_CALL: Gas = 20_000_000_000_000;
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
    pub contract_id: AccountId,
    pub function_id: String,
    pub cadence: String,
    pub recurring: bool,
    pub deposit: U128,
    pub gas: Gas,
    pub arguments: Vec<u8>,
}

#[ext_contract(ext_croncat)]
pub trait ExtCroncat {
    fn get_slot_tasks(&self, offset: Option<u64>) -> (Vec<Base64VecU8>, U128);
    fn get_tasks(&self, slot: Option<U128>, from_index: Option<U64>, limit: Option<U64>) -> Vec<Task>;
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
    fn compute_callback(
        &self,
        #[callback]
        #[serializer(borsh)]
        info: (bool, AccountId, U64, U64, [u64; 2], U128, U64, U64, U128, U128, U128, U128, U64, U64, U64, U128),
    );
}

// GOALs:
// create a contract the has full cron CRUD operations managed within this contract
// contract utility is sample idea of an indexer: keep track of info numbers in a "timeseries"

// NOTE: The series could be updated to support OHLCV, Sums, MACD, etc...

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    HourlyBalanceSeries,
    HourlyQueueSeries,
    HourlySlotsSeries,
    DailyBalanceSeries,
    DailyQueueSeries,
    DailySlotsSeries,
}

#[derive(Default, BorshDeserialize, BorshSerialize, Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TickItem {
    t: u64,  // point in time
    x: Option<u128>, // value at time
    y: Option<u128>, // value at time
    z: Option<u128>, // value at time
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CrudContract {
    // tick: sums over 1hr of data, holding 30 days of hourly items
    hourly_balances: Vector<TickItem>,
    hourly_queues: Vector<TickItem>,
    hourly_slots: Vector<TickItem>,
    // tick: sums over 1 day of data, holding 1 year of daily items
    daily_balances: Vector<TickItem>,
    daily_queues: Vector<TickItem>,
    daily_slots: Vector<TickItem>,
    // Cron task hash, default will be running at the hourly scale
    task_hash: Option<Base64VecU8>,
    // Cron manager account (manager_v1.croncat.near)
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
            hourly_balances: Vector::new(StorageKeys::HourlyBalanceSeries),
            hourly_queues: Vector::new(StorageKeys::HourlyQueueSeries),
            hourly_slots: Vector::new(StorageKeys::HourlySlotsSeries),
            daily_balances: Vector::new(StorageKeys::HourlyBalanceSeries),
            daily_queues: Vector::new(StorageKeys::HourlyQueueSeries),
            daily_slots: Vector::new(StorageKeys::HourlySlotsSeries),
            task_hash: None,
            cron,
        }
    }

    /// Returns the time series of data for hourly, daily
    ///
    /// ```bash
    /// near view crosscontract.testnet get_series
    /// ```
    pub fn get_series(&self) -> (Vec<TickItem>, Vec<TickItem>, Vec<TickItem>, Vec<TickItem>, Vec<TickItem>, Vec<TickItem>) {
        (
            self.hourly_balances.to_vec(),
            self.hourly_queues.to_vec(),
            self.hourly_slots.to_vec(),
            self.daily_balances.to_vec(),
            self.daily_queues.to_vec(),
            self.daily_slots.to_vec(),
        )
    }

    /// Compute: CrudContract Heartbeat
    /// Used to compute this time periods hourly/daily
    /// This fn can be called a varying intervals to compute rolling window time series data.
    ///
    /// ```bash
    /// near call crosscontract.testnet compute '{}' --accountId YOUR_ACCOUNT.testnet
    /// ```
    pub fn compute(&mut self) -> Promise {
        ext_croncat::get_info(
            &self.cron.clone().expect(ERR_NO_CRON_CONFIGURED),
            env::attached_deposit(),
            GAS_FOR_SCHEDULE_CALL,
        )
        .then(ext::compute_callback(
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_COMPUTE_CALLBACK,
        ))
    }

    /// Get the task hash, and store in state
    /// NOTE: This method helps contract understand remaining task balance, in case more is needed to continue running.
    /// NOTE: This could handle things about the task, or have logic about changing the task in some way.
    #[private]
    pub fn compute_callback(&mut self, #[callback] info: (bool, AccountId, U64, U64, [u64; 2], U128, U64, U64, U128, U128, U128, U128, U64, U64, U64, U128)) {
        // compute the current intervals
        let block_ts = env::block_timestamp();
        let rem_threshold = 60_000;
        let rem_hour = core::cmp::max(block_ts % MILLISECONDS_IN_HOUR, 1);
        let rem_day = core::cmp::max(block_ts % MILLISECONDS_IN_DAY, 1);
        log!("REMS: {:?} {:?}", rem_hour, rem_day);
        log!(
            "LENS: {:?} {:?} {:?} {:?} {:?} {:?}",
            self.hourly_balances.len(),
            self.hourly_queues.len(),
            self.hourly_slots.len(),
            self.daily_balances.len(),
            self.daily_queues.len(),
            self.daily_slots.len(),
        );

        // Le stuff frem le responsi
        let (
            _,
            _,
            agent_active_queue,
            agent_pending_queue,
            _,
            _,
            slots,
            tasks,
            available_balance,
            staked_balance,
            _,
            _,
            _,
            slot_granularity,
            _,
            balance,
        ) = info;

        // get some data value, at a point in time
        // I chose a stupid value, but one that changes over time. This can be changed to account balances, token prices, anything that changes over time.
        let hour_balance = TickItem {
            t: block_ts / NANOS,
            x: Some(balance.0),
            y: Some(available_balance.0),
            z: Some(staked_balance.0),
        };
        log!("New HR Balance: {:?}", hour_balance);
        
        // More ticks
        let hour_queue = TickItem {
            t: block_ts / NANOS,
            x: Some(agent_active_queue.0 as u128),
            y: Some(agent_pending_queue.0 as u128),
            z: None,
        };
        let hour_slots = TickItem {
            t: block_ts / NANOS,
            x: Some(slots.0 as u128),
            y: Some(tasks.0 as u128),
            z: Some(slot_granularity.0 as u128),
        };

        // compute for each interval match, made a small buffer window to make sure the computed value doesnt get computed too far out of range
        self.hourly_balances.push(&hour_balance);
        self.hourly_queues.push(&hour_queue);
        self.hourly_slots.push(&hour_slots);

        // trim to max
        if self.hourly_balances.len() > 744 {
            // 31 days of hours (24*31)
            // TODO: Change this to unshift lol
            self.hourly_balances.pop();
            self.hourly_queues.pop();
            self.hourly_slots.pop();
        }

        // daily average across last 1hr of data including NEW
        if rem_day <= rem_threshold {
            // 86_400_000
            let total_day_ticks: u64 = 24;
            let end_index = self.daily_balances.len();
            let start_index = end_index - total_day_ticks;
            let mut hour_balance_tick = TickItem {
                t: block_ts / NANOS,
                x: Some(0),
                y: Some(0),
                z: Some(0),
            };
            let mut hour_queue_tick = TickItem {
                t: block_ts / NANOS,
                x: Some(0),
                y: Some(0),
                z: None,
            };
            let mut hour_slots_tick = TickItem {
                t: block_ts / NANOS,
                x: Some(0),
                y: Some(0),
                z: Some(0),
            };

            // minus 1 for current number above
            for i in start_index..end_index {
                if let Some(tick) = self.daily_balances.get(i) {
                    // Aggregate tick numbers
                    hour_balance_tick.x = if tick.x.is_some() { Some(hour_balance_tick.x.unwrap_or(0) + tick.x.unwrap_or(0)) } else { hour_balance_tick.x };
                    hour_balance_tick.y = if tick.y.is_some() { Some(hour_balance_tick.y.unwrap_or(0) + tick.y.unwrap_or(0)) } else { hour_balance_tick.y };
                    hour_balance_tick.z = if tick.z.is_some() { Some(hour_balance_tick.z.unwrap_or(0) + tick.z.unwrap_or(0)) } else { hour_balance_tick.z };
                };
                if let Some(tick) = self.hourly_queues.get(i) {
                    // Aggregate tick numbers
                    hour_queue_tick.x = if tick.x.is_some() { Some(hour_queue_tick.x.unwrap_or(0) + tick.x.unwrap_or(0)) } else { hour_queue_tick.x };
                    hour_queue_tick.y = if tick.y.is_some() { Some(hour_queue_tick.y.unwrap_or(0) + tick.y.unwrap_or(0)) } else { hour_queue_tick.y };
                };
                if let Some(tick) = self.hourly_slots.get(i) {
                    // Aggregate tick numbers
                    hour_slots_tick.x = if tick.x.is_some() { Some(hour_slots_tick.x.unwrap_or(0) + tick.x.unwrap_or(0)) } else { hour_slots_tick.x };
                    hour_slots_tick.y = if tick.y.is_some() { Some(hour_slots_tick.y.unwrap_or(0) + tick.y.unwrap_or(0)) } else { hour_slots_tick.y };
                    hour_slots_tick.z = if tick.z.is_some() { Some(hour_slots_tick.z.unwrap_or(0) + tick.z.unwrap_or(0)) } else { hour_slots_tick.z };
                };
            }

            self.daily_balances.push(&hour_balance_tick);
            self.daily_balances.push(&hour_queue_tick);
            self.daily_balances.push(&hour_slots_tick);

            // trim to max
            if end_index > 1825 {
                // 5 years of days (365*5)
                self.daily_balances.pop();
            }
        }
    }

    /// Create a new scheduled task, registering the "compute" method with croncat
    ///
    /// ```bash
    /// near call crosscontract.testnet schedule '{ "function_id": "compute", "period": "0 0 * * * *" }' --accountId YOUR_ACCOUNT.testnet
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
            Some(GAS_FOR_COMPUTE_CALL), // 30 Tgas
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
        // NOTE: fix this! serialization is not working
        let hash = self.task_hash.clone().expect(ERR_NO_TASK_CONFIGURED);
        log!(
            "TASK HASH: {:?} {:?} {}",
            &hash,
            serde_json::to_string(&hash).unwrap(),
            serde_json::to_string(&hash).unwrap()
        );
        ext_croncat::get_task(
            // hash,
            serde_json::to_string(&hash).unwrap().to_string(),
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
        // NOTE: Check remaining balance here
        // NOTE: Could have logic to another callback IF the balance is running low
        task
    }

    /// Get the stats!
    ///
    /// ```bash
    /// near call crosscontract.testnet status
    /// ```
    pub fn stats(&self) -> (u64, u64, Option<Base64VecU8>, Option<AccountId>) {
        (
            self.hourly_balances.len(),
            self.daily_balances.len(),
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

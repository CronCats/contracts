use near_sdk::{
    ext_contract,
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Serialize, Deserialize},
    collections::Vector,
    json_types::{Base64VecU8, U128},
    env, log, near_bindgen, AccountId, Gas, BorshStorageKey, Promise, PanicOnDefault,
};

near_sdk::setup_alloc!();

pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const NANOS: u64 = 1_000_000;
pub const MILLISECONDS_IN_MINUTE: u64 = 60_000;
pub const MILLISECONDS_IN_HOUR: u64 = 3_600_000;
pub const MILLISECONDS_IN_DAY: u64 = 86_400_000;

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
    task_hash: Base64VecU8
  );
  fn status_callback(
    &self,
    #[callback]
    #[serializer(borsh)]
    task: Option<Task>
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
    t: u64, // point in time
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
}

#[near_bindgen]
impl CrudContract {
    /// ```bash
    /// near call crosscontract.testnet new --accountId YOUR_ACCOUNT.testnet
    /// ```
    #[init]
    pub fn new() -> Self {
      assert!(!env::state_exists(), "The contract is already initialized");
      CrudContract {
        minutely: Vector::new(StorageKeys::MinutelySeries),
        hourly: Vector::new(StorageKeys::HourlySeries),
        daily: Vector::new(StorageKeys::DailySeries),
        task_hash: None,
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
        self.daily.to_vec()
      )
    }

    /// Tick: CrudContract Heartbeat
    /// Used to compute this time periods minutely/hourly/daily
    /// This fn can be called a varying intervals to compute rolling window time series data.
    ///
    /// near call crosscontract.testnet tick '{}' --accountId YOUR_ACCOUNT.testnet
    pub fn tick(&mut self) {
      // compute the current intervals
      let block_ts = env::block_timestamp();
      let validator_num = env::validator_total_stake();
      let rem_minute = core::cmp::max(block_ts % MILLISECONDS_IN_MINUTE, 1);
      let rem_hour = core::cmp::max(block_ts % MILLISECONDS_IN_HOUR, 1);
      let rem_day = core::cmp::max(block_ts % MILLISECONDS_IN_DAY, 1);
      log!("REMS: {:?} {:?} {:?}", rem_minute, rem_hour, rem_day);
      log!("LENS: {:?} {:?} {:?}", self.minutely.len(), self.hourly.len(), self.daily.len());

      // get some data value, at a point in time
      // I chose a stupid value, but one that changes over time. This can be changed to account balances, token prices, anything that changes over time.
      let minute_tick = TickItem {
        t: block_ts / NANOS,
        v: validator_num,
      };
      log!("New Tick: {:?}", minute_tick);

      // compute for each interval match, made a small buffer window to make sure the computed value doesnt get computed too far out of range
      if rem_minute <= 10_000 { // 60_000
        self.minutely.push(&minute_tick);

        // trim to max
        if self.minutely.len() > 1440 { // 24 hours of minutes (24*60)
          self.minutely.pop();
        }
      }

      // hourly average across last 1hr of data including NEW
      if rem_hour <= 40_000 { // 3_600_000
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
        if end_index > 744 { // 31 days of hours (24*31)
          self.hourly.pop();
        }
      }

      // daily average across last 1hr of data including NEW
      if rem_hour <= 120_000 { // 86_400_000
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
        if end_index > 1825 { // 5 years of days (365*5)
          self.daily.pop();
        }
      }
    }

    /// Create a new scheduled task, registering the "tick" method with croncat
    ///
    /// near call crosscontract.testnet schedule '{ "function_id": "tick", "period": "0 */1 * * * *" }' --accountId YOUR_ACCOUNT.testnet
    pub fn schedule(&mut self, function_id: String, period: String) -> Promise {
      // TODO: safety checks
      ext_croncat::create_task(
        env::current_account_id(),
        function_id,
        period,
        Some(true),
        Some(U128::from(0)),
        Some(240000000000000000),
        None,
        &env::current_account_id(),
        0,
        env::prepaid_gas() / 3
      ).then(
        ext::schedule_callback(
          &env::current_account_id(),
          0,
          env::prepaid_gas() / 3
        )
      )
    }

    /// Get the task hash, and store in state
    #[result_serializer(borsh)]
    #[private]
    pub fn schedule_callback(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        task_hash: Base64VecU8
    ) {
        log!("schedule_callback task_hash {:?}", &task_hash);
        self.task_hash = Some(task_hash);
    }

    /// Update a scheduled task using a known task hash, passing new updateable parameters. MUST be owner!
    /// NOTE: There's much more you could do here with the parameters, just showing an example of period update.
    ///
    /// near call crosscontract.testnet update '{ "period": "0 0 */1 * * *", "task_hash": "r2JvrGPvDkFUuqdF4x1+L93aYKGmgp4GqXT4UAK3AE4=" }' --accountId YOUR_ACCOUNT.testnet
    pub fn update(&mut self, period: String, task_hash: Base64VecU8) -> Promise {
      // TODO: safety checks
      ext_croncat::update_task(
        task_hash,
        Some(period),
        None,
        None,
        None,
        None,
        &env::current_account_id(),
        0,
        env::prepaid_gas()
      )
    }

    /// Remove a scheduled task using a known hash. MUST be owner!
    ///
    /// near call crosscontract.testnet remove '{ "task_hash": "r2JvrGPvDkFUuqdF4x1+L93aYKGmgp4GqXT4UAK3AE4=" }' --accountId YOUR_ACCOUNT.testnet
    pub fn remove(&mut self, task_hash: Base64VecU8) -> Promise {
      // TODO: safety checks
      ext_croncat::remove_task(
        task_hash,
        &env::current_account_id(),
        0,
        env::prepaid_gas()
      )
    }

    /// Get the task status, including remaining balance & etc.
    /// Useful for automated on-chain task management! This method could be scheduled as well, and manage re-funding tasks or changing tasks on new data.
    ///
    /// near call crosscontract.testnet status
    pub fn status(&self) -> Promise {
      ext_croncat::get_task(
        self.task_hash.clone().expect("No task hash found, need to schedule a cron task to set and get it."),
        &env::current_account_id(),
        0,
        env::prepaid_gas() / 3
      ).then(
        ext::schedule_callback(
          &env::current_account_id(),
          0,
          env::prepaid_gas() / 3
        )
      )
    }

    /// Get the task hash, and store in state
    /// NOTE: This method helps contract understand remaining task balance, in case more is needed to continue running.
    /// NOTE: This could handle things about the task, or have logic about changing the task in some way.
    #[result_serializer(borsh)]
    #[private]
    pub fn status_callback(
        &self,
        #[callback]
        #[serializer(borsh)]
        task: Option<Task>
    ) -> Option<Task> {
      task
    }
}

// // use the attribute below for unit tests
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use near_sdk::MockedBlockchain;
//     use near_sdk::{testing_env, VMContext};

//     // part of writing unit tests is setting up a mock context
//     // in this example, this is only needed for env::log in the contract
//     // this is also a useful list to peek at when wondering what's available in env::*
//     fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
//         VMContext {
//             current_account_id: "alice.testnet".to_string(),
//             signer_account_id: "robert.testnet".to_string(),
//             signer_account_pk: vec![0, 1, 2],
//             predecessor_account_id: "jane.testnet".to_string(),
//             input,
//             block_index: 0,
//             block_timestamp: 0,
//             account_balance: 0,
//             account_locked_balance: 0,
//             storage_usage: 0,
//             attached_deposit: 0,
//             prepaid_gas: 10u64.pow(18),
//             random_seed: vec![0, 1, 2],
//             is_view,
//             output_data_receivers: vec![],
//             epoch_height: 19,
//         }
//     }

//     // mark individual unit tests with #[test] for them to be registered and fired
//     #[test]
//     fn increment() {
//         // set up the mock context into the testing environment
//         let context = get_context(vec![], false);
//         testing_env!(context);
//         // instantiate a contract variable with the counter at zero
//         let mut contract = Counter { val: 0 };
//         contract.increment();
//         println!("Value after increment: {}", contract.get_num());
//         // confirm that we received 1 when calling get_num
//         assert_eq!(1, contract.get_num());
//     }
// }

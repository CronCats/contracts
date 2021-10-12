<div align="center">
  <h1>
    Cron.near Contracts
  </h1>
  <p>
  But i really really wanted to name this repo "crontracts"
  </p>
</div>

## Building
Run:
```bash
./build.sh
```

## Testing
To test run:
```bash
cargo test --package manager -- --nocapture
```

## Scripts
The following scripts automate a lot of the tedious setup for contracts, and allow for quick deployments and setup. These are scripted versions of the example commands below.

NOTE: See each script to change the main `NEAR_ACCT` to configure to an account you have testnet keys.

Run:
```bash
./scripts/clear_all.sh
./scripts/create_and_deploy.sh
./scripts/simple_bootstrap.sh
```

## Create testnet subaccounts
Next, create a NEAR testnet account with [Wallet](https://wallet.testnet.near.org).

Set an environment variable to use in these examples. For instance, if your test account is `you.testnet` set it like so:

```bash
export NEAR_ACCT=you.testnet
```

(**Windows users**: please look into using `set` instead of `export`, surrounding the environment variable in `%` instead of beginning with `$`, and using escaped double-quotes `\"` where necessary instead of the single-quotes provided in these instructions.)

Create subaccounts:

```bash
near create-account cron.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create-account counter.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create-account agent.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create-account user.$NEAR_ACCT --masterAccount $NEAR_ACCT
near create-account crud.$NEAR_ACCT --masterAccount $NEAR_ACCT
```

**Note**: if changes are made to the contract and it needs to be redeployed, it's a good idea to delete and recreate the subaccount like so:

```bash
near delete cron.$NEAR_ACCT $NEAR_ACCT && near create-account cron.$NEAR_ACCT --masterAccount $NEAR_ACCT
near delete agent.$NEAR_ACCT $NEAR_ACCT && near create-account agent.$NEAR_ACCT --masterAccount $NEAR_ACCT
near delete crud.$NEAR_ACCT $NEAR_ACCT && near create-account crud.$NEAR_ACCT --masterAccount $NEAR_ACCT
```

## Contract Interaction

```
# Deploy New
near deploy --wasmFile ./res/manager.wasm --accountId cron.$NEAR_ACCT --initFunction new --initArgs '{}'
near deploy --wasmFile ./res/rust_counter_tutorial.wasm --accountId counter.$NEAR_ACCT
near deploy --wasmFile ./res/cross_contract.wasm --accountId crud.$NEAR_ACCT --initFunction new --initArgs '{"cron": "cron.in.testnet"}'

# Deploy Migration
near deploy --wasmFile ./res/manager.wasm --accountId cron.$NEAR_ACCT --initFunction migrate_state --initArgs '{}'

# Schedule "ticks" that help provide in-contract BPS calculation
near call cron.$NEAR_ACCT create_task '{"contract_id": "cron.'$NEAR_ACCT'","function_id": "tick","cadence": "0 0 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId cron.$NEAR_ACCT --amount 10

# Tasks
near call cron.$NEAR_ACCT create_task '{"contract_id": "counter.'$NEAR_ACCT'","function_id": "increment","cadence": "0 */5 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId counter.$NEAR_ACCT --amount 10

near view cron.$NEAR_ACCT get_task '{"task_hash": "r2JvrGPvDkFUuqdF4x1+L93aYKGmgp4GqXT4UAK3AE4="}'

near call cron.$NEAR_ACCT remove_task '{"task_hash": "r2JvrGPvDkFUuqdF4x1+L93aYKGmgp4GqXT4UAK3AE4="}' --accountId counter.$NEAR_ACCT

near view cron.$NEAR_ACCT get_tasks '{"offset": 999}'

near call cron.$NEAR_ACCT proxy_call --accountId agent.$NEAR_ACCT

near view cron.$NEAR_ACCT get_all_tasks

# Agents
near call cron.$NEAR_ACCT register_agent '{"payable_account_id": "user.'$NEAR_ACCT'"}' --accountId agent.$NEAR_ACCT

near call cron.$NEAR_ACCT update_agent '{"payable_account_id": "user.'$NEAR_ACCT'"}' --accountId agent.$NEAR_ACCT

near call cron.$NEAR_ACCT unregister_agent --accountId agent.$NEAR_ACCT --amount 0.000000000000000000000001

near view cron.$NEAR_ACCT get_agent '{"pk": "ed25519:AGENT_PUBLIC_KEY"}'

near call cron.$NEAR_ACCT withdraw_task_balance --accountId agent.$NEAR_ACCT

# ------------------------------------
# Counter Interaction
near view counter.$NEAR_ACCT get_num
near call counter.$NEAR_ACCT increment --accountId $NEAR_ACCT
near call counter.$NEAR_ACCT decrement --accountId $NEAR_ACCT

# ------------------------------------
# Cross-Contract Interaction
near view crud.$NEAR_ACCT get_series
near view crud.$NEAR_ACCT stats
near call crud.$NEAR_ACCT tick --accountId $NEAR_ACCT
near call crud.$NEAR_ACCT schedule '{ "function_id": "tick", "period": "0 */5 * * * *" }' --accountId crud.$NEAR_ACCT --gas 300000000000000 --amount 5
near call crud.$NEAR_ACCT update '{ "period": "0 0 */1 * * *" }' --accountId crud.$NEAR_ACCT --gas 300000000000000 --amount 5
near call crud.$NEAR_ACCT remove --accountId crud.$NEAR_ACCT
near call crud.$NEAR_ACCT status --accountId crud.$NEAR_ACCT
```

## Changelog

### `0.4.0`

Mainnet preparation, convenience methods, multi-agent support

### `0.2.0`

Audit recommendations implemented, bug fixes. Watch audit here: https://youtu.be/KPAQbFz8RnE

### `0.1.0`

Testnet version stable, gas efficiencies, initial full spec complete

### `0.0.1`

Initial setup
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
```

**Note**: if changes are made to the contract and it needs to be redeployed, it's a good idea to delete and recreate the subaccount like so:

```bash
near delete cron.$NEAR_ACCT $NEAR_ACCT && near create-account cron.$NEAR_ACCT --masterAccount $NEAR_ACCT
near delete agent.$NEAR_ACCT $NEAR_ACCT && near create-account agent.$NEAR_ACCT --masterAccount $NEAR_ACCT
```

## Contract Interaction

```
# Deploy
near deploy --wasmFile ./res/manager.wasm --accountId cron.$NEAR_ACCT --initFunction new --initArgs '{}'
near deploy --wasmFile ./res/rust_counter_tutorial.wasm --accountId counter.$NEAR_ACCT

# Tasks
near call cron.$NEAR_ACCT create_task '{"contract_id": "counter.'$NEAR_ACCT'","function_id": "increment","cadence": "*/10 * * * * *","recurring": true,"deposit": 10,"gas": 2400000000000}' --accountId counter.$NEAR_ACCT --amount 10

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
```

## Changelog

### `0.1.0`

Testnet version stable, gas efficiencies, initial full spec complete

### `0.0.1`

Initial setup
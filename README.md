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

## Contract Interaction

```
# Deploy
near deploy --wasmFile "./res/manager.wasm" --accountId cron.in.testnet
near deploy --wasmFile "./res/rust_counter_tutorial.wasm" --accountId counter.in.testnet

near call cron.in.testnet new --accountId cron.in.testnet

# Tasks
near call cron.in.testnet create_task '{"contract_id": "counter.in.testnet","function_id": "increment","cadence": "@epoch","recurring": true,"fn_allowance": 0,"gas_allowance": 2400000000000}' --accountId counter.in.testnet --amount 100

near call cron.in.testnet remove_task '{"task_hash": [103, 236, 102, 89, 237, 21, 121, 198, 170, 6, 169, 157, 21, 187, 168, 103, 238, 34, 117, 4, 54, 193, 147, 190, 226, 221, 35, 190, 247, 208, 221, 144]}' --accountId counter.in.testnet

near view cron.in.testnet get_task '{"task_hash": [103, 236, 102, 89, 237, 21, 121, 198, 170, 6, 169, 157, 21, 187, 168, 103, 238, 34, 117, 4, 54, 193, 147, 190, 226, 221, 35, 190, 247, 208, 221, 144]}' --accountId counter.in.testnet

near view cron.in.testnet get_task '{"task_hash": [95, 18, 65, 232, 183, 57, 173, 22, 28, 105, 176, 87, 148, 18, 111, 148, 106, 12, 90, 109, 32, 223, 36, 101, 68, 227, 216, 115, 160, 172, 77, 194]}' --accountId counter.in.testnet

near call cron.in.testnet get_tasks --accountId agent.in.testnet

near call cron.in.testnet proxy_call --accountId agent.in.testnet

# Agents
near call cron.in.testnet register_agent '{"payable_account_id": "user.in.testnet"}' --accountId agent.in.testnet

near call cron.in.testnet update_agent '{"payable_account_id": "user.in.testnet"}' --accountId agent.in.testnet

near call cron.in.testnet unregister_agent --accountId agent.in.testnet

near view cron.testnet get_agent '{"pk": "ed25519:AGENT_PUBLIC_KEY"}' --accountId YOU.testnet

near call cron.in.testnet withdraw_task_balance --accountId agent.in.testnet

# ------------------------------------
# Counter Interaction
near view counter.in.testnet get_num
near call counter.in.testnet increment --accountId in.testnet
near call counter.in.testnet decrement --accountId in.testnet
```

## Changelog

### `0.0.1`

Initial setup
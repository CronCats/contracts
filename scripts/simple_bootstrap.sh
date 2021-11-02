#!/bin/bash
# Uncomment the desired network
export NEAR_ENV=testnet
# export NEAR_ENV=mainnet
# export NEAR_ENV=guildnet
# export NEAR_ENV=betanet

export FACTORY=testnet
# export FACTORY=near
# export FACTORY=registrar

export MAX_GAS=300000000000000

if [ -z ${NEAR_ACCT+x} ]; then
  export NEAR_ACCT=cron.$FACTORY
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=manager_v1.$NEAR_ACCT
export REWARDS_ACCOUNT_ID=rewards_v1.$NEAR_ACCT
export COUNTER_ACCOUNT_ID=counter.$NEAR_ACCT
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export USER_ACCOUNT_ID=user.$NEAR_ACCT
export CRUD_ACCOUNT_ID=crudcross.$NEAR_ACCT
export DAO_ACCOUNT_ID=croncat.sputnikv2.$FACTORY

# Check all configs first
near view $CRON_ACCOUNT_ID version
near view $CRON_ACCOUNT_ID get_info

# # UnPause the manager (only turn on for rapid testing, otherwise the main flow will go through DAO)
# near call $CRON_ACCOUNT_ID update_settings '{ "paused": false }' --accountId $CRON_ACCOUNT_ID --gas $MAX_GAS

# # Assign ownership to the DAO
near call $CRON_ACCOUNT_ID update_settings '{ "owner_id": "'$DAO_ACCOUNT_ID'", "paused": false }' --accountId $CRON_ACCOUNT_ID --gas $MAX_GAS

# Register the "tick" task, as the base for regulating BPS
near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$CRON_ACCOUNT_ID'","function_id": "tick","cadence": "0 0 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $CRON_ACCOUNT_ID --amount 10

# Register "increment" task, for doing basic cross-contract test
near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$COUNTER_ACCOUNT_ID'","function_id": "increment","cadence": "0 */1 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $COUNTER_ACCOUNT_ID --amount 10

# Register "tick" from crud example
near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$CRUD_ACCOUNT_ID'","function_id": "tick","cadence": "0 */5 * * * *","recurring": true,"deposit": "0","gas": 10000000000000}' --accountId $CRUD_ACCOUNT_ID --amount 10

# Check the tasks were setup right:
near view $CRON_ACCOUNT_ID get_tasks

# Register 1 agent
near call $CRON_ACCOUNT_ID register_agent '{"payable_account_id": "'$USER_ACCOUNT_ID'"}' --accountId $USER_ACCOUNT_ID --amount 0.00484
near view $CRON_ACCOUNT_ID get_agent '{"account_id": "'$USER_ACCOUNT_ID'"}'
near call $CRON_ACCOUNT_ID register_agent '{"payable_account_id": "'$AGENT_ACCOUNT_ID'"}' --accountId $AGENT_ACCOUNT_ID --amount 0.00484
near view $CRON_ACCOUNT_ID get_agent '{"account_id": "'$AGENT_ACCOUNT_ID'"}'

# # Agent check for first task
near view $CRON_ACCOUNT_ID get_agent_tasks '{"account_id": "'$USER_ACCOUNT_ID'"}'
# near view $CRON_ACCOUNT_ID get_slot_tasks

# # Call the first task
# near call $CRON_ACCOUNT_ID proxy_call --accountId $USER_ACCOUNT_ID --gas $MAX_GAS

# # Pause the manager
# near call $CRON_ACCOUNT_ID update_settings '{ "paused": true }' --accountId $CRON_ACCOUNT_ID --gas $MAX_GAS

# Insane battery of tasks to test multiple agents
near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$COUNTER_ACCOUNT_ID'","function_id": "increment","cadence": "0 */2 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $COUNTER_ACCOUNT_ID --amount 0.5
near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$COUNTER_ACCOUNT_ID'","function_id": "increment","cadence": "0 */3 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $COUNTER_ACCOUNT_ID --amount 0.5
near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$COUNTER_ACCOUNT_ID'","function_id": "increment","cadence": "0 */4 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $COUNTER_ACCOUNT_ID --amount 0.5
near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$COUNTER_ACCOUNT_ID'","function_id": "increment","cadence": "0 */5 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $COUNTER_ACCOUNT_ID --amount 0.5
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$COUNTER_ACCOUNT_ID'","function_id": "increment","cadence": "0 */6 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $COUNTER_ACCOUNT_ID --amount 0.5
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$COUNTER_ACCOUNT_ID'","function_id": "increment","cadence": "0 */7 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $COUNTER_ACCOUNT_ID --amount 0.5
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$COUNTER_ACCOUNT_ID'","function_id": "increment","cadence": "0 */8 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $COUNTER_ACCOUNT_ID --amount 0.5
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$COUNTER_ACCOUNT_ID'","function_id": "increment","cadence": "0 */9 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $COUNTER_ACCOUNT_ID --amount 0.5

echo ""
echo "Start your agents, waiting 1m to onboard another agent..."
echo ""

sleep 1m
near call $CRON_ACCOUNT_ID tick --accountId $CRON_ACCOUNT_ID

echo "Cron $NEAR_ENV Bootstrap Complete"
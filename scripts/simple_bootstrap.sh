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
  export NEAR_ACCT=ion.$NEAR_ENV
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=cron.$NEAR_ACCT
export COUNTER_ACCOUNT_ID=counter.$NEAR_ACCT
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export USER_ACCOUNT_ID=user.$NEAR_ACCT
export CRUD_ACCOUNT_ID=crud.$NEAR_ACCT
export DAO_ACCOUNT_ID=dao.sputnikv2.$NEAR_ENV

# Check all configs first
near view $CRON_ACCOUNT_ID version
near view $CRON_ACCOUNT_ID get_info

# # Register the "tick" task, as the base for regulating BPS
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$CRON_ACCOUNT_ID'","function_id": "tick","cadence": "0 0 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId $CRON_ACCOUNT_ID --amount 10

# # Register "increment" task, for doing basic cross-contract test
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "counter.'$NEAR_ACCT'","function_id": "increment","cadence": "0 */5 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId counter.$NEAR_ACCT --amount 10

# # Register "tick" from crud example
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "crud.'$NEAR_ACCT'","function_id": "tick","cadence": "0 */10 * * * *","recurring": true,"deposit": "0","gas": 10000000000000}' --accountId $CRON_ACCOUNT_ID --amount 10

# # Check the tasks were setup right:
# near view $CRON_ACCOUNT_ID get_all_tasks

# # Register 1 agent
# near call $CRON_ACCOUNT_ID register_agent '{"payable_account_id": "'$USER_ACCOUNT_ID'"}' --accountId $USER_ACCOUNT_ID --amount 0.00242
near view $CRON_ACCOUNT_ID get_agent '{"account_id": "'$USER_ACCOUNT_ID'"}'

# Agent check for first task
near view $CRON_ACCOUNT_ID get_tasks '{"account_id": "'$USER_ACCOUNT_ID'"}'
# near view $CRON_ACCOUNT_ID get_tasks

# Call the first task
near call $CRON_ACCOUNT_ID proxy_call --accountId $USER_ACCOUNT_ID --gas $MAX_GAS

echo "Cron Bootstrap Complete"
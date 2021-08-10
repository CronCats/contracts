#!/bin/bash
# Uncomment the desired network
export NEAR_ENV=testnet
# export NEAR_ENV=mainnet
# export NEAR_ENV=guildnet
# export NEAR_ENV=betanet

export FACTORY=testnet
# export FACTORY=near
# export FACTORY=registrar

if [ -z ${NEAR_ACCT+x} ]; then
  export NEAR_ACCT=you.testnet
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=cron.$NEAR_ACCT
export COUNTER_ACCOUNT_ID=counter.$NEAR_ACCT
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export USER_ACCOUNT_ID=user.$NEAR_ACCT
export CRUD_ACCOUNT_ID=crud.$NEAR_ACCT
export DAO_ACCOUNT_ID=dao.sputnikv2.testnet

# Register the "tick" task, as the base for regulating BPS
near call cron.$NEAR_ACCT create_task '{"contract_id": "cron.'$NEAR_ACCT'","function_id": "tick","cadence": "0 0 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId cron.$NEAR_ACCT --amount 10

# Register "increment" task, for doing basic cross-contract test
near call cron.$NEAR_ACCT create_task '{"contract_id": "counter.'$NEAR_ACCT'","function_id": "increment","cadence": "0 */5 * * * *","recurring": true,"deposit": "0","gas": 2400000000000}' --accountId counter.$NEAR_ACCT --amount 10

# Register "tick" from crud example
near call cron.$NEAR_ACCT create_task '{"contract_id": "cron.'$NEAR_ACCT'","function_id": "tick","cadence": "0 */10 * * * *","recurring": true,"deposit": "0","gas": 10000000000000}' --accountId cron.$NEAR_ACCT --amount 10

# Check the tasks were setup right:
near view cron.$NEAR_ACCT get_all_tasks

echo "Cron Bootstrap Complete"
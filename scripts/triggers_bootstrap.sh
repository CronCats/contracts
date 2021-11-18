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
  export NEAR_ACCT=weicat.$FACTORY
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=manager_v1.$NEAR_ACCT
export REWARDS_ACCOUNT_ID=rewards.$NEAR_ACCT
export COUNTER_ACCOUNT_ID=counter.$NEAR_ACCT
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export USER_ACCOUNT_ID=user.$NEAR_ACCT
export CRUD_ACCOUNT_ID=crudcross.$NEAR_ACCT
export VIEWS_ACCOUNT_ID=views.$NEAR_ACCT
export DAO_ACCOUNT_ID=croncat.sputnikv2.$FACTORY

# Register an agent
# near call $CRON_ACCOUNT_ID register_agent '{"payable_account_id": "'$AGENT_ACCOUNT_ID'"}' --accountId $AGENT_ACCOUNT_ID --amount 0.00484

# Create a task
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "counter.weicat.testnet","function_id": "increment","cadence": "0 0 * 12 * *","recurring": true,"deposit": "0","gas": 4000000000000}' --accountId $USER_ACCOUNT_ID --amount 10

# get hash from above
near call $CRON_ACCOUNT_ID create_trigger '{"contract_id": "views.weicat.testnet","function_id": "get_a_boolean","task_hash":"McJZp4jUfZkwlGGVn0T5wpGVAPvsvMbHmO1R9rv8tco="}' --accountId $USER_ACCOUNT_ID --amount 0.000017

# VSB8VDqS8QgmTTCTuvt5q9BiXLUnv77AJxwBWZIO7U4=
near view $CRON_ACCOUNT_ID get_triggers '{"from_index": "0", "limit": "10"}'

# do a view check
near view $VIEWS_ACCOUNT_ID get_a_boolean

# Make the actual proxy view+call
near call $CRON_ACCOUNT_ID proxy_conditional_call '{"trigger_hash": "VSB8VDqS8QgmTTCTuvt5q9BiXLUnv77AJxwBWZIO7U4"}' --accountId $AGENT_ACCOUNT_ID --gas 300000000000000

sleep 1m

# Do AGAIN just in case we were on odd minute
near call $CRON_ACCOUNT_ID proxy_conditional_call '{"trigger_hash": "VSB8VDqS8QgmTTCTuvt5q9BiXLUnv77AJxwBWZIO7U4"}' --accountId $AGENT_ACCOUNT_ID --gas 300000000000000

echo "Trigger sample complete"

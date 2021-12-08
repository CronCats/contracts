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
  export NEAR_ACCT=croncat.$FACTORY
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=manager_v1.$NEAR_ACCT
export COUNTER_ACCOUNT_ID=counter.$NEAR_ACCT
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export USER_ACCOUNT_ID=user.$NEAR_ACCT
export CRUD_ACCOUNT_ID=crud.$NEAR_ACCT
export DAO_ACCOUNT_ID=croncat.sputnikv2.$FACTORY
# export DAO_ACCOUNT_ID=croncat.sputnik-dao.$FACTORY

# # Change ownership to DAO
# near call $CRON_ACCOUNT_ID update_settings '{"owner_id": "'$DAO_ACCOUNT_ID'"}' --accountId $CRON_ACCOUNT_ID

# # Submit proposal to change a configuration setting (Example: Change agent fee)
# ARGS=`echo "{ \"agent_fee\": \"1000000000000000000000\" }" | base64`
# FIXED_ARGS=`echo $ARGS | tr -d '\r' | tr -d ' '`
# near call $DAO_ACCOUNT_ID add_proposal '{"proposal": {"description": "Change cron manager settings, see attached arguments for what is changing", "kind": {"FunctionCall": {"receiver_id": "'$CRON_ACCOUNT_ID'", "actions": [{"method_name": "update_settings", "args": "'$FIXED_ARGS'", "deposit": "0", "gas": "20000000000000"}]}}}}' --accountId $NEAR_ACCT --amount 0.1

# # Check all configs
near view $CRON_ACCOUNT_ID version
near view $CRON_ACCOUNT_ID get_info
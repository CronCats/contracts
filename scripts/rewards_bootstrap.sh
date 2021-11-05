#!/bin/bash
# Uncomment the desired network
# export NEAR_ENV=testnet
export NEAR_ENV=mainnet
# export NEAR_ENV=guildnet
# export NEAR_ENV=betanet

# export FACTORY=testnet
export FACTORY=near
# export FACTORY=registrar

export MAX_GAS=300000000000000

if [ -z ${NEAR_ACCT+x} ]; then
  export NEAR_ACCT=croncat.$FACTORY
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=manager_v1.$NEAR_ACCT
export REWARDS_ACCOUNT_ID=rewards.$NEAR_ACCT
export COUNTER_ACCOUNT_ID=counter.$NEAR_ACCT
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export USER_ACCOUNT_ID=user.$NEAR_ACCT
export CRUD_ACCOUNT_ID=crudcross.$NEAR_ACCT
export DAO_ACCOUNT_ID=croncat.sputnikv2.$FACTORY

# Check all configs first
near view $REWARDS_ACCOUNT_ID version

# Config things
# near call $REWARDS_ACCOUNT_ID update_settings '{"pixelpet_account_id": "pixeltoken.near"}' --accountId $REWARDS_ACCOUNT_ID --gas $MAX_GAS

# Test automated distro
# 0.0251 payment needed
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$REWARDS_ACCOUNT_ID'","function_id": "pet_check_task_ownership","cadence": "0 */1 * * * *","recurring": true,"deposit": "0","gas": 120000000000000, "arguments": "eyJ0YXNrX2hhc2giOiIxZjVMZHBqdGtLUHJRN2dvUDNUSUNXQ2Q5ZjZwMzhYNWNibXJqam9HdHVvPSJ9"}' --accountId $USER_ACCOUNT_ID --amount 0.0251 --gas $MAX_GAS

# Do quick test of pet distro
# near call $REWARDS_ACCOUNT_ID pet_check_task_ownership '{"task_hash": "TBD"}' --accountId $USER_ACCOUNT_ID --gas $MAX_GAS

echo "Cron $NEAR_ENV Bootstrap Complete"

#!/bin/bash
# This file is used for starting a fresh set of all contracts & configs
set -e

if [ -d "res" ]; then
  echo ""
else
  mkdir res
fi

cd "`dirname $0`"

if [ -z "$KEEP_NAMES" ]; then
  export RUSTFLAGS='-C link-arg=-s'
else
  export RUSTFLAGS=''
fi

# build the things
cargo build --all --target wasm32-unknown-unknown --release
cp ../target/wasm32-unknown-unknown/release/*.wasm ./res/

# Uncomment the desired network
export NEAR_ENV=mainnet

export FACTORY=near

if [ -z ${NEAR_ACCT+x} ]; then
  # you will need to change this to something you own
  export NEAR_ACCT=croncat.$FACTORY
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export MAX_GAS=300000000000000

export CRON_ACCOUNT_ID=manager_v1.$NEAR_ACCT
export REWARDS_ACCOUNT_ID=rewards.$NEAR_ACCT
export DAO_ACCOUNT_ID=croncat.sputnik-dao.near

######
# NOTE: All commands below WORK, just have them off for safety.
######

## clear and recreate all accounts
# near delete $CRON_ACCOUNT_ID $NEAR_ACCT
# near delete $REWARDS_ACCOUNT_ID $NEAR_ACCT


## create all accounts
# near create-account $CRON_ACCOUNT_ID --masterAccount $NEAR_ACCT --initialBalance 10
# near create-account $REWARDS_ACCOUNT_ID --masterAccount $NEAR_ACCT --initialBalance 3


# Deploy all the contracts to their rightful places
# near deploy --wasmFile ./res/manager.wasm --accountId $CRON_ACCOUNT_ID --initFunction new --initArgs '{}'
# near deploy --wasmFile ./res/rewards.wasm --accountId $REWARDS_ACCOUNT_ID --initFunction new --initArgs '{"cron_account_id": "'$CRON_ACCOUNT_ID'", "dao_account_id": "'$DAO_ACCOUNT_ID'"}'


# # Assign ownership to the DAO
# near call $CRON_ACCOUNT_ID update_settings '{ "owner_id": "'$DAO_ACCOUNT_ID'", "paused": true }' --accountId $CRON_ACCOUNT_ID --gas $MAX_GAS

# # Configure initial requirements
# near call $REWARDS_ACCOUNT_ID update_settings '{"pixelpet_account_id": "pixeltoken.near"}' --accountId $REWARDS_ACCOUNT_ID --gas $MAX_GAS

# RE:Deploy all the contracts to their rightful places
# near deploy --wasmFile ./res/manager.wasm --accountId $CRON_ACCOUNT_ID
# near deploy --wasmFile ./res/rewards.wasm --accountId $REWARDS_ACCOUNT_ID


# Check all configs first
# near view $CRON_ACCOUNT_ID version
# near view $CRON_ACCOUNT_ID get_info
near view $REWARDS_ACCOUNT_ID version

echo "Mainnet Deploy Complete"
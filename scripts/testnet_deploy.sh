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
export NEAR_ENV=testnet

export FACTORY=testnet

if [ -z ${NEAR_ACCT+x} ]; then
  # you will need to change this to something you own
  export NEAR_ACCT=croncat.$FACTORY
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=manager_v1.$NEAR_ACCT
export REWARDS_ACCOUNT_ID=rewards.$NEAR_ACCT
export COUNTER_ACCOUNT_ID=counter.$NEAR_ACCT
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export USER_ACCOUNT_ID=user.$NEAR_ACCT
export CRUD_ACCOUNT_ID=crud.$NEAR_ACCT
export VIEWS_ACCOUNT_ID=views.$NEAR_ACCT
export DAO_ACCOUNT_ID=dao.sputnikv2.$FACTORY

######
# NOTE: All commands below WORK, just have them off for safety.
######

## clear and recreate all accounts
# near delete $CRON_ACCOUNT_ID $NEAR_ACCT
# near delete $COUNTER_ACCOUNT_ID $NEAR_ACCT
# near delete $AGENT_ACCOUNT_ID $NEAR_ACCT
# near delete $USER_ACCOUNT_ID $NEAR_ACCT
# near delete $CRUD_ACCOUNT_ID $NEAR_ACCT
near delete $VIEWS_ACCOUNT_ID $NEAR_ACCT


## create all accounts
# near create-account $CRON_ACCOUNT_ID --masterAccount $NEAR_ACCT
# near create-account $COUNTER_ACCOUNT_ID --masterAccount $NEAR_ACCT
# near create-account $AGENT_ACCOUNT_ID --masterAccount $NEAR_ACCT
# near create-account $USER_ACCOUNT_ID --masterAccount $NEAR_ACCT
# near create-account $CRUD_ACCOUNT_ID --masterAccount $NEAR_ACCT
near create-account $VIEWS_ACCOUNT_ID --masterAccount $NEAR_ACCT


# Deploy all the contracts to their rightful places
# near deploy --wasmFile ./res/manager.wasm --accountId $CRON_ACCOUNT_ID --initFunction new --initArgs '{}'
# near deploy --wasmFile ./res/rust_counter_tutorial.wasm --accountId $COUNTER_ACCOUNT_ID
# near deploy --wasmFile ./res/cross_contract.wasm --accountId $CRUD_ACCOUNT_ID --initFunction new --initArgs '{"cron": "'$CRON_ACCOUNT_ID'"}'
near deploy --wasmFile ./res/views.wasm --accountId $VIEWS_ACCOUNT_ID


# RE:Deploy all the contracts to their rightful places
# near deploy --wasmFile ./res/manager.wasm --accountId $CRON_ACCOUNT_ID
# near deploy --wasmFile ./res/rust_counter_tutorial.wasm --accountId $COUNTER_ACCOUNT_ID
# near deploy --wasmFile ./res/cross_contract.wasm --accountId $CRUD_ACCOUNT_ID
# near deploy --wasmFile ./res/rewards.wasm --accountId $REWARDS_ACCOUNT_ID

near view $CRON_ACCOUNT_ID version
near view $CRON_ACCOUNT_ID get_info

echo "Testnet Deploy Complete"
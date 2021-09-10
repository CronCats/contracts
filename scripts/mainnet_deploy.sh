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
cp target/wasm32-unknown-unknown/release/*.wasm ./res/

# Uncomment the desired network
export NEAR_ENV=mainnet

export FACTORY=near

if [ -z ${NEAR_ACCT+x} ]; then
  # you will need to change this to something you own
  export NEAR_ACCT=cron.near
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=manager_v1.$NEAR_ACCT
export DAO_ACCOUNT_ID=croncat.sputnikdao.near

######
# NOTE: All commands below WORK, just have them off for safety.
######

## clear and recreate all accounts
# near delete $CRON_ACCOUNT_ID $NEAR_ACCT


## create all accounts
# near create-account $CRON_ACCOUNT_ID --masterAccount $NEAR_ACCT


# Deploy all the contracts to their rightful places
# near deploy --wasmFile ./res/manager.wasm --accountId $CRON_ACCOUNT_ID --initFunction new --initArgs '{}'


# RE:Deploy all the contracts to their rightful places
# near deploy --wasmFile ./res/manager.wasm --accountId $CRON_ACCOUNT_ID
# near view $CRON_ACCOUNT_ID version

echo "Testnet Deploy Complete"
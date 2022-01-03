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
export AIRDROP_ACCOUNT_ID=airdrop.$NEAR_ACCT
export DAO_ACCOUNT_ID=croncat.sputnikv2.$FACTORY

# Check all configs first
near view $AIRDROP_ACCOUNT_ID stats

# Config things
near call $AIRDROP_ACCOUNT_ID add_manager '{"account_id": "'$DAO_ACCOUNT_ID'"}' --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS
near call $AIRDROP_ACCOUNT_ID add_account '{"account_id": "'$NEAR_ACCT'"}' --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS
near call $AIRDROP_ACCOUNT_ID add_account '{"account_id": "cron.testnet"}' --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS

# Test automated distro
near call $AIRDROP_ACCOUNT_ID multisend '{"transfer_type": "FungibleToken", "amount": "500000000000000000000000"}' --amount 1 --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS

# End result
near view $AIRDROP_ACCOUNT_ID stats

echo "Cron $NEAR_ENV Airdrop Complete"

#!/bin/bash
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
# export NEAR_ENV=mainnet
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
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export DAO_ACCOUNT_ID=croncat.sputnikv2.$FACTORY
export FT_ACCOUNT_ID=ft.$NEAR_ACCT
export NFT_ACCOUNT_ID=nft.$NEAR_ACCT

# near delete $AIRDROP_ACCOUNT_ID $NEAR_ACCT
# near create-account $AIRDROP_ACCOUNT_ID --masterAccount $NEAR_ACCT
# near deploy --wasmFile ./res/airdrop.wasm --accountId $AIRDROP_ACCOUNT_ID --initFunction new --initArgs '{"ft_account_id": "'$FT_ACCOUNT_ID'"}'
# # near deploy --wasmFile ./res/airdrop.wasm --accountId $AIRDROP_ACCOUNT_ID

# # Setup & Deploy FT & NFT
# near delete $FT_ACCOUNT_ID $NEAR_ACCT
# near delete $NFT_ACCOUNT_ID $NEAR_ACCT
# near create-account $FT_ACCOUNT_ID --masterAccount $NEAR_ACCT
# near create-account $NFT_ACCOUNT_ID --masterAccount $NEAR_ACCT
# near deploy --wasmFile ../res/fungible_token.wasm --accountId $FT_ACCOUNT_ID --initFunction new --initArgs '{ "owner_id": "'$AIRDROP_ACCOUNT_ID'", "total_supply": "100000000000000000", "metadata": { "spec": "ft-1.0.0", "name": "Airdrop Token", "symbol": "ADP", "decimals": 18 } }'
# near deploy --wasmFile ../res/non_fungible_token.wasm --accountId $NFT_ACCOUNT_ID --initFunction new_default_meta --initArgs '{"owner_id": "'$AIRDROP_ACCOUNT_ID'"}'
# near view $FT_ACCOUNT_ID ft_balance_of '{"account_id": "'$AIRDROP_ACCOUNT_ID'"}'

# # Check all configs first
# near view $AIRDROP_ACCOUNT_ID stats

# near call $AIRDROP_ACCOUNT_ID reset_index --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS

# # Config things
# near call $AIRDROP_ACCOUNT_ID add_manager '{"account_id": "'$AIRDROP_ACCOUNT_ID'"}' --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS
# near call $AIRDROP_ACCOUNT_ID add_manager '{"account_id": "'$DAO_ACCOUNT_ID'"}' --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS

# # # create a bunch of test users to airdrop to
# TOTAL_USERS=10
# for (( e=0; e<=TOTAL_USERS; e++ ))
# do
#   TMP_USER="user_${e}.$NEAR_ACCT"

#   near delete $TMP_USER $NEAR_ACCT
#   near create-account $TMP_USER --masterAccount $NEAR_ACCT
#   near call $FT_ACCOUNT_ID storage_deposit --accountId $TMP_USER --amount 0.00484

#   near call $AIRDROP_ACCOUNT_ID add_account '{"account_id": "'$TMP_USER'"}' --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS
# done

# Test automated distro
# near call $AIRDROP_ACCOUNT_ID multisend '{"transfer_type": "Near", "amount": "500000000000000000000000"}' --amount 10 --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS
# Finish testing FT setup
# near call $AIRDROP_ACCOUNT_ID multisend '{"transfer_type": "FungibleToken", "amount": "5"}' --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS
# TODO: Finish testing NFT setup
# near call $AIRDROP_ACCOUNT_ID multisend '{"transfer_type": "NonFungibleToken", "amount": "1"}' --accountId $AIRDROP_ACCOUNT_ID --gas $MAX_GAS

# # Register "multisend" task, which will get triggered back to back until the pagination is complete
# near call $CRON_ACCOUNT_ID remove_task '{"task_hash": "UK1+xizXmG974zooHOH8VvkoNT1vOz3PqJpk3A/lCbo="}' --accountId $AIRDROP_ACCOUNT_ID
# # # Args are for 0.5 near transfer per account
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$AIRDROP_ACCOUNT_ID'","function_id": "multisend","cadence": "0 * * * * *","recurring": true,"deposit": "2500000000000000000000000","gas": 200000000000000, "arguments": "eyJ0cmFuc2Zlcl90eXBlIjogIk5lYXIiLCAiYW1vdW50IjogIjUwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMCJ9"}' --accountId $AIRDROP_ACCOUNT_ID --amount 8
# # # Args are for 0.5 Fungible Token transfer per account
# near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$AIRDROP_ACCOUNT_ID'","function_id": "multisend","cadence": "1 * * * * *","recurring": true,"deposit": "2500000000000000000000000","gas": 200000000000000, "arguments": "eyJ0cmFuc2Zlcl90eXBlIjogIkZ1bmdpYmxlVG9rZW4iLCAiYW1vdW50IjogIjUwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMCJ9"}' --accountId $AIRDROP_ACCOUNT_ID --amount 8
# # # TODO: Args are for a Non Fungible Token transfer per account
# # near call $CRON_ACCOUNT_ID create_task '{"contract_id": "'$AIRDROP_ACCOUNT_ID'","function_id": "multisend","cadence": "0 * * * * *","recurring": true,"deposit": "2500000000000000000000000","gas": 200000000000000, "arguments": ""}' --accountId $AIRDROP_ACCOUNT_ID --amount 8

# # Call proxy_call to trigger multisend
# # near call $CRON_ACCOUNT_ID register_agent '{"payable_account_id": "'$AGENT_ACCOUNT_ID'"}' --accountId $AGENT_ACCOUNT_ID --amount 0.00484

# sleep 1m
# near call $CRON_ACCOUNT_ID proxy_call --accountId $AGENT_ACCOUNT_ID --gas $MAX_GAS
# near call $CRON_ACCOUNT_ID proxy_call --accountId $AGENT_ACCOUNT_ID --gas $MAX_GAS
# near call $CRON_ACCOUNT_ID proxy_call --accountId $AGENT_ACCOUNT_ID --gas $MAX_GAS
# near call $CRON_ACCOUNT_ID proxy_call --accountId $AGENT_ACCOUNT_ID --gas $MAX_GAS
# near call $CRON_ACCOUNT_ID proxy_call --accountId $AGENT_ACCOUNT_ID --gas $MAX_GAS
# near call $CRON_ACCOUNT_ID proxy_call --accountId $AGENT_ACCOUNT_ID --gas $MAX_GAS

# End result
near view $AIRDROP_ACCOUNT_ID stats

echo "Cron $NEAR_ENV Airdrop Complete"

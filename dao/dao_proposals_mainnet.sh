#!/bin/bash
set -e
MASTER_ACC=cron.near
DAO_ROOT_ACC=sputnik-dao.near
DAO_NAME=croncat
DAO_ACCOUNT=$DAO_NAME.$DAO_ROOT_ACC
CRON_ACCOUNT=manager_v1.croncat.near

export NEAR_ENV=mainnet

## CRONCAT Launch proposal (unpause)
# ARGS=`echo "{ \"paused\": false }" | base64`
# FIXED_ARGS=`echo $ARGS | tr -d '\r' | tr -d ' '`
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "Unpause the croncat manager contract to enable cron tasks", "kind": {"FunctionCall": {"receiver_id": "'$CRON_ACCOUNT'", "actions": [{"method_name": "update_settings", "args": "'$FIXED_ARGS'", "deposit": "0", "gas": "50000000000000"}]}}}}' --accountId $MASTER_ACC --amount 0.1

## CRONCAT config change proposal
# ARGS=`echo "{ \"agents_eject_threshold\": \"600\" }" | base64`
# FIXED_ARGS=`echo $ARGS | tr -d '\r' | tr -d ' '`
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "Change agent kick length to 10 hours", "kind": {"FunctionCall": {"receiver_id": "'$CRON_ACCOUNT'", "actions": [{"method_name": "update_settings", "args": "'$FIXED_ARGS'", "deposit": "0", "gas": "50000000000000"}]}}}}' --accountId $MASTER_ACC --amount 0.1

## CRONCAT Launch proposal: TICK Task
# ARGS=`echo "{\"contract_id\": \"$CRON_ACCOUNT\",\"function_id\": \"tick\",\"cadence\": \"0 0 * * * *\",\"recurring\": true,\"deposit\": \"0\",\"gas\": 2400000000000}" | base64`
# FIXED_ARGS=`echo $ARGS | tr -d '\r' | tr -d ' '`
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "Create cron task to manage TICK method to handle agents every hour for 1 year", "kind": {"FunctionCall": {"receiver_id": "'$CRON_ACCOUNT'", "actions": [{"method_name": "create_task", "args": "'$FIXED_ARGS'", "deposit": "7000000000000000000000000", "gas": "50000000000000"}]}}}}' --accountId $MASTER_ACC --amount 0.1

## payout proposal
# PAYOUT_AMT=1000000000000000000000000
# PAYOUT_ACCT=prod.near
# near call $DAO_ACCOUNT add_proposal '{"proposal": { "description": "Payout", "kind": { "Transfer": { "token_id": "", "receiver_id": "'$PAYOUT_ACCT'", "amount": "'$PAYOUT_AMT'" } } } }' --accountId $MASTER_ACC --amount 0.1

## add member to one of our roles
# ROLE=founders
# ROLE=applications
# ROLE=agents
# ROLE=commanders
# NEW_MEMBER=prod.near
# near call $DAO_ACCOUNT add_proposal '{ "proposal": { "description": "Welcome '$NEW_MEMBER' to the '$ROLE' team", "kind": { "AddMemberToRole": { "member_id": "'$NEW_MEMBER'", "role": "'$ROLE'" } } } }' --accountId $MASTER_ACC --amount 0.1
# near call $DAO_ACCOUNT add_proposal '{ "proposal": { "description": "Remove '$NEW_MEMBER' from '$ROLE' for non-availability", "kind": { "RemoveMemberFromRole": { "member_id": "'$NEW_MEMBER'", "role": "'$ROLE'" } } } }' --accountId $MASTER_ACC --amount 0.1

## CRONCAT Scheduling proposal example
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "demo croncat test", "kind": {"FunctionCall": {"receiver_id": "crud.in.testnet", "actions": [{"method_name": "tick", "args": "e30=", "deposit": "0", "gas": "20000000000000"}]}}}}' --accountId $MASTER_ACC --amount 0.1

## --------------------------------
## METAPOOL STAKING
## --------------------------------
METAPOOL_ACCT=meta-pool.near
# Stake
# 10 NEAR
# STAKE_AMOUNT_NEAR=10000000000000000000000000
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "Stake funds from croncat dao to metapool", "kind": {"FunctionCall": {"receiver_id": "'$METAPOOL_ACCT'", "actions": [{"method_name": "deposit_and_stake", "args": "e30=", "deposit": "'$STAKE_AMOUNT_NEAR'", "gas": "20000000000000"}]}}}}' --accountId $MASTER_ACC --amount 0.1

# # Unstake all (example: 9xVyewMkzxHfRGtx3EyG82mXX8CfPXLJeW4Xo2y6PpXX)
# ARGS=`echo "{ \"amount\": \"10000000000000000000000000\" }" | base64`
# FIXED_ARGS=`echo $ARGS | tr -d '\r' | tr -d ' '`
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "Unstake all funds from metapool to croncat dao", "kind": {"FunctionCall": {"receiver_id": "'$METAPOOL_ACCT'", "actions": [{"method_name": "unstake", "args": "'$FIXED_ARGS'", "deposit": "0", "gas": "20000000000000"}]}}}}' --accountId $MASTER_ACC --amount 0.1

# # Withdraw balance back (example: EKZqArNzsjq9hpYuYt37Y59qU1kmZoxguLwRH2RnDELd)
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "Withdraw unstaked funds from metapool to croncat dao", "kind": {"FunctionCall": {"receiver_id": "'$METAPOOL_ACCT'", "actions": [{"method_name": "withdraw_unstaked", "args": "e30=", "deposit": "0", "gas": "20000000000000"}]}}}}' --accountId $MASTER_ACC --amount 0.1

## --------------------------------
## PARAS.ID NFTs
## --------------------------------
##
## NOTE: To mint a series, first upload artwork via secret account and get the ipfs hash, as there is no way to upload via DAO
PARAS_ACCT=x.paras.near

# [img, reference]
# https://cdn.paras.id/tr:w-0.8/bafybeigs5r2g3kpz7ucjwtbbadqbif6mwm7a27ga67cou6tme4zj4gvnha
# FOUNDER: {"status":1,"data":["ipfs://bafybeigs5r2g3kpz7ucjwtbbadqbif6mwm7a27ga67cou6tme4zj4gvnha","ipfs://bafybeidy77pjqzurxoanbtuulzinou5tbxoomlbgacumxcxdxht5m2afbq"]}
# COMMANDER: {"status":1,"data":["ipfs://bafybeibzaeouzmifvkjwtpbl5sccp7z3ej5yqyef7yeeb3p5ikpnjtraeu","ipfs://bafybeie5te5ylzgyx76ks2gjolzszb6obfwhd6zply77rfk7yq722xf3nq"]}
# AGENT: {"status":1,"data":["ipfs://bafybeidha5l6fv7jm4mtgg5v6lplp4pmxx4zxxw4jaiq7fxqsx7nnyed3m","ipfs://bafybeibkvkazyu5oed6k6mp7fn7fnm7yv3krvolhfh4zwmciruvt55ardm"]}
# APP: {"status":1,"data":["ipfs://bafybeiae3i55h377miym7yrovyu5b6f75us56zdkkxi3y4gvmaoiotxiva","ipfs://bafybeietne5xe7ad2hsye5ltpdrfri2hpxgheukutv76kuydqki2f4hbam"]}
# CHEFS: {"status":1,"data":["ipfs://bafybeig5bbumicsi6g2hu5gpzcukxke6rcxpcmc7gpgk725t5spxft7i64","ipfs://bafybeiadifgrnym5b6q6ktbd3qh7se2wtu4lcr4ta3fj2waarigifujdva"]}

# Create the Series! We have 5 Series today: Founders, Agents, Applications, Commanders, Chefs
# nft_create_series
# {
#   "creator_id": "croncat.sputnik-dao.near",
#   "token_metadata": {
#     "title": "Croncat",
#     "media": "bafybeid7ytiw7yea...",
#     "reference": "bafybeih7jhlqs7g65...",
#     "copies": 10
#   },
#   "price": null,
#   "royalty": {
#     "croncat.sputnik-dao.near": 700
#   }
# }
# ARGS=`echo "{ \"creator_id\": \"$DAO_ACCOUNT\", \"token_metadata\": { \"title\": \"Croncat DAO Commander\", \"media\": \"bafybeibzaeouzmifvkjwtpbl5sccp7z3ej5yqyef7yeeb3p5ikpnjtraeu\", \"reference\": \"bafybeie5te5ylzgyx76ks2gjolzszb6obfwhd6zply77rfk7yq722xf3nq\", \"copies\": 37 }, \"price\": null, \"royalty\": { \"$DAO_ACCOUNT\": 700 } }" | base64`
# FIXED_ARGS=`echo $ARGS | tr -d '\r' | tr -d ' '`
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "Create NFT series for Commanders of Croncat DAO", "kind": {"FunctionCall": {"receiver_id": "'$PARAS_ACCT'", "actions": [{"method_name": "nft_create_series", "args": "'$FIXED_ARGS'", "deposit": "4500000000000000000000", "gas": "20000000000000"}]}}}}' --accountId $MASTER_ACC --amount 1


# Mint an NFT to a user
# Get the token_series_id from the above create series result
# FOUNDER: 40444
# COMMANDER: 40982
# AGENT: 40984
# APP: 40985
# CHEF: 40986
# 
# nft_mint
# {
#   "token_series_id": "39640",
#   "receiver_id": "account.near"
# }

# SERIES_ID=40444
# RECEIVER=account.near
# ARGS=`echo "{ \"token_series_id\": \"$SERIES_ID\", \"receiver_id\":  \"$RECEIVER\" }" | base64`
# FIXED_ARGS=`echo $ARGS | tr -d '\r' | tr -d ' '`
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "Mint NFT for Commander '$RECEIVER'", "kind": {"FunctionCall": {"receiver_id": "'$PARAS_ACCT'", "actions": [{"method_name": "nft_mint", "args": "'$FIXED_ARGS'", "deposit": "7400000000000000000000", "gas": "90000000000000"}]}}}}' --accountId $MASTER_ACC --amount 0.1

## --------------------------------
## Vote
## --------------------------------
## NOTE: Examples setup as needed, adjust variables for use cases.
# near view $DAO_ACCOUNT get_policy
# near call $DAO_ACCOUNT act_proposal '{"id": 0, "action" :"VoteApprove"}' --accountId $MASTER_ACC  --gas 300000000000000
# near call $DAO_ACCOUNT act_proposal '{"id": 0, "action" :"VoteReject"}' --accountId $MASTER_ACC  --gas 300000000000000
# near call $DAO_ACCOUNT act_proposal '{"id": 0, "action" :"VoteRemove"}' --accountId $MASTER_ACC  --gas 300000000000000

# # Loop All Action IDs and submit action
# vote_actions=(72 73 74 75 76 77 78 79)
# for (( e=0; e<=${#vote_actions[@]} - 1; e++ ))
# do
#   # action="VoteApprove"
#   # action="VoteReject"
#   action="VoteRemove"
#   SUB_ACT_PROPOSAL=`echo "{\"id\": ${vote_actions[e]}, \"action\" :\"${action}\"}"`
#   echo "Payload ${SUB_ACT_PROPOSAL}"

#   near call $DAO_ACCOUNT act_proposal '{"id": '${vote_actions[e]}', "action" :"'${action}'"}' --accountId $MASTER_ACC  --gas 300000000000000
# done
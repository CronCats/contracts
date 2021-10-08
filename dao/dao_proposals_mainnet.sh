set -e
MASTER_ACC=cron.near
DAO_ROOT_ACC=sputnik-dao.near
DAO_NAME=croncat
DAO_ACCOUNT=$DAO_NAME.$DAO_ROOT_ACC

export NEAR_ENV=mainnet

## NOTE: Examples setup as needed, adjust variables for use cases.
# near view $DAO_ACCOUNT get_policy
# near call $DAO_ACCOUNT act_proposal '{"id": 0, "action" :"VoteApprove"}' --accountId $MASTER_ACC  --gas 300000000000000
# near call $DAO_ACCOUNT act_proposal '{"id": 0, "action" :"VoteReject"}' --accountId $MASTER_ACC  --gas 300000000000000
# near call $DAO_ACCOUNT act_proposal '{"id": 0, "action" :"VoteRemove"}' --accountId $MASTER_ACC  --gas 300000000000000

## payout proposal
# near call $DAO_ACCOUNT add_proposal '{"proposal": { "description": "", "kind": { "Transfer": { "token_id": "", "receiver_id": "cron.near", "amount": "1000000000000000000000000" } } } }' --accountId $MASTER_ACC --amount 0.1

## add member to one of our roles
# ROLE=founders
# ROLE=applications
# ROLE=agents
# ROLE=commanders
# NEW_MEMBER=prod.near
# near call $DAO_ACCOUNT add_proposal '{ "proposal": { "description": "Welcome '$NEW_MEMBER' to the '$ROLE' team", "kind": { "AddMemberToRole": { "member_id": "'$NEW_MEMBER'", "role": "'$ROLE'" } } } }' --accountId $MASTER_ACC --amount 0.1

## CRONCAT Scheduling proposal example
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "demo croncat test", "kind": {"FunctionCall": {"receiver_id": "crud.in.testnet", "actions": [{"method_name": "tick", "args": "e30=", "deposit": "0", "gas": "20000000000000"}]}}}}' --accountId $MASTER_ACC --amount 0.1

## METAPOOL STAKING
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
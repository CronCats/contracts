set -e
MASTER_ACC=cron.near
DAO_ROOT_ACC=sputnik-dao.near
DAO_NAME=croncat
DAO_ACCOUNT=$DAO_NAME.$DAO_ROOT_ACC

##Change NEAR_ENV between mainnet, testnet and betanet
# export NEAR_ENV=testnet
export NEAR_ENV=mainnet

# NOTE: Examples setup as needed, adjust variables for use cases.
# near view $DAO_ACCOUNT get_policy
# near call $DAO_ACCOUNT act_proposal '{"id": 0, "action" :"VoteApprove"}' --accountId $MASTER_ACC  --gas 300000000000000
# near call $DAO_ACCOUNT act_proposal '{"id": 0, "action" :"VoteReject"}' --accountId $MASTER_ACC  --gas 300000000000000
# near call $DAO_ACCOUNT act_proposal '{"id": 0, "action" :"VoteRemove"}' --accountId $MASTER_ACC  --gas 300000000000000

# CRONCAT Scheduling proposal example
# near call $DAO_ACCOUNT add_proposal '{"proposal": {"description": "demo croncat test", "kind": {"FunctionCall": {"receiver_id": "crud.in.testnet", "actions": [{"method_name": "tick", "args": "e30=", "deposit": "0", "gas": "20000000000000"}]}}}}' --accountId $MASTER_ACC --amount 1
set -e
MASTER_ACC=in.testnet
CONTRACT_ACC=dao.$MASTER_ACC
DAO_NAME=dao

near delete $CONTRACT_ACC $MASTER_ACC

export NODE_ENV=testnet
export POLICY='{
  "roles": [],
  "proposal_bond": "10000000000000000000000",
  "proposal_period": "604800000000000",
  "bounty_bond": "1000000000000000000000000",
  "bounty_forgiveness_period": "86400000000000"
}}'

# near create-account $CONTRACT_ACC --masterAccount $MASTER_ACC --initialBalance 20
# near deploy --wasmFile=res/sputnikdao2.wasm --initFunction new --initArgs '{"config": {"name": "metabuild", "purpose": "Hackathon DAO", "metadata":""}, "policy": "'$POLICY'"}' --initGas 300000000000000 --accountId $CONTRACT_ACC
# ARGS=`echo "{\"config\":  {\"name\": \"metabuild\", \"purpose\": \"Hackathon DAO\", \"metadata\":\"\"}, \"policy\": $POLICY"`
# read input
# near call $MASTER_ACC create "{\"name\": \"$DAO_NAME\", \"args\": \"$ARGS\"}" --accountId $CONTRACT_ACC --amount 20 --gas 150000000000000
# near deploy --wasmFile=res/sputnikdao2.wasm --initFunction new --initArgs $ARGS  --accountId $CONTRACT_ACC --initGas 150000000000000
# near deploy --wasmFile=res/sputnikdao2.wasm --initFunction new --initArgs "{\"config\":  {\"name\": \"metabuild\", \"purpose\": \"Hackathon DAO\", \"metadata\":\"\"}, \"policy\": $POLICY" --accountId $CONTRACT_ACC

# near deploy --wasmFile=res/sputnikdao2.wasm --accountId $CONTRACT_ACC
# near call $CONTRACT_ACC new '{"config": {"name": "metabuild", "purpose": "Hackathon DAO", "metadata":""}, "policy": "'$POLICY'"}' --accountId $CONTRACT_ACC
near view $CONTRACT_ACC get_policy
echo "DAO succesfully deployed!"

##redeploy only
#near deploy $CONTRACT_ACC --wasmFile=res/sputnikdao2.wasm  --accountId $MASTER_ACC

#save last deployment 
#cp ./res/sputnikdao2.wasm ./res/sputnikdao2.`date +%F.%T`.wasm
#date +%F.%T
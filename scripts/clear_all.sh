#!/bin/bash
# Uncomment the desired network
export NEAR_ENV=testnet
# export NEAR_ENV=mainnet
# export NEAR_ENV=guildnet
# export NEAR_ENV=betanet

export FACTORY=testnet
# export FACTORY=near
# export FACTORY=registrar

if [ -z ${NEAR_ACCT+x} ]; then
  export NEAR_ACCT=you.testnet
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=cron.$NEAR_ACCT
export COUNTER_ACCOUNT_ID=counter.$NEAR_ACCT
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export USER_ACCOUNT_ID=user.$NEAR_ACCT
export CRUD_ACCOUNT_ID=crud.$NEAR_ACCT
export DAO_ACCOUNT_ID=dao.sputnikv2.testnet

# clear and recreate all accounts
near delete $CRON_ACCOUNT_ID $NEAR_ACCT
near delete $COUNTER_ACCOUNT_ID $NEAR_ACCT
near delete $AGENT_ACCOUNT_ID $NEAR_ACCT
near delete $USER_ACCOUNT_ID $NEAR_ACCT
near delete $CRUD_ACCOUNT_ID $NEAR_ACCT

echo "Clear Complete"
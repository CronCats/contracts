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
  export NEAR_ACCT=cron.$FACTORY
else
  export NEAR_ACCT=$NEAR_ACCT
fi

export CRON_ACCOUNT_ID=manager_v1.$NEAR_ACCT
export REWARDS_ACCOUNT_ID=rewards.$NEAR_ACCT
export COUNTER_ACCOUNT_ID=counter.$NEAR_ACCT
export AGENT_ACCOUNT_ID=agent.$NEAR_ACCT
export USER_ACCOUNT_ID=user.$NEAR_ACCT
export CRUD_ACCOUNT_ID=crudcross.$NEAR_ACCT
export VIEWS_ACCOUNT_ID=views.$NEAR_ACCT
export DAO_ACCOUNT_ID=croncat.sputnikv2.$FACTORY

# clear and recreate all accounts
near delete $CRON_ACCOUNT_ID $NEAR_ACCT
near delete $REWARDS_ACCOUNT_ID $NEAR_ACCT
near delete $COUNTER_ACCOUNT_ID $NEAR_ACCT
near delete $AGENT_ACCOUNT_ID $NEAR_ACCT
near delete $USER_ACCOUNT_ID $NEAR_ACCT
near delete $CRUD_ACCOUNT_ID $NEAR_ACCT
near delete $VIEWS_ACCOUNT_ID $NEAR_ACCT

echo "Clear Complete"
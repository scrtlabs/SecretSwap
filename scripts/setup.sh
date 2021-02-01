#!/bin/bash

set -xe


function secretcli() {
  export docker_name=secretdev
  docker exec "$docker_name" secretcli "$@";
}

function wait_for_tx() {
  until (secretcli q tx "$1"); do
      sleep 5
  done
}

export SGX_MODE=SW
export deployer_name=a
export wasm_path=/root/code/build

export deployer_address=$(secretcli keys show -a $deployer_name)
echo "Deployer address: '$deployer_address'"

secretcli tx compute store "${wasm_path}/secretswap_token.wasm" --from "$deployer_name" --gas 2000000 -b block -y
token_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
token_code_hash=$(secretcli query compute list-code | jq '.[-1]."data_hash"')
echo "Stored token: '$token_code_id', '$token_code_hash'"

secretcli tx compute store "${wasm_path}/secretswap_factory.wasm" --from "$deployer_name" --gas 2000000 -b block -y
factory_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
echo "Stored factory: '$factory_code_id'"

secretcli tx compute store "${wasm_path}/secretswap_pair.wasm" --from "$deployer_name" --gas 2000000 -b block -y
pair_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
pair_code_hash=$(secretcli query compute list-code | jq '.[-1]."data_hash"')
echo "Stored pair: '$pair_code_id', '$pair_code_hash'"

echo "Deploying token..."
label=$(date +"%T")

export TX_HASH=$(
  secretcli tx compute instantiate $token_code_id '{"admin": "'$deployer_address'", "symbol": "ETH", "decimals": 6, "initial_balances": [{"address": "'$deployer_address'", "amount": "100000000000"}], "prng_seed": "YWE", "name": "test"}' --from $deployer_name --gas 1500000 --label $label -b block -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

token_addr=$(secretcli query compute list-contract-by-code $token_code_id | jq '.[-1].address')
echo "Token address: '$token_addr'"

label=$(date +"%T")
export TX_HASH=$(
  secretcli tx compute instantiate $factory_code_id '{"pair_code_id": '$pair_code_id', "pair_code_hash": '$pair_code_hash', "token_code_id": '$token_code_id', "token_code_hash": '$token_code_hash', "prng_seed": "YWE"}' --label $label --from $deployer_name -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

factory_contract=$(secretcli query compute list-contract-by-code $factory_code_id | jq '.[-1].address')
echo "Factory address: '$factory_contract'"

secretcli tx compute execute --label $label '{"create_pair": {"asset_infos": [{"native_token": {"denom": "uscrt"}},{"token": {"contract_addr": '$token_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}]}}' --from $deployer_name -y --gas 1500000 -b block

pair_contract=$(secretcli query compute list-contract-by-code $pair_code_id | jq '.[-1].address')
echo "Pair contract address: '$pair_contract'"

lptoken=$(secretcli query compute list-contract-by-code $token_code_id | jq '.[-1].address')
echo "LP Token address: '$lptoken'"

secretcli tx compute execute $(echo "$token_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract', "amount": "1000000000"}}' -b block -y --from $deployer_name
secretcli tx compute execute $(echo "$pair_contract" | tr -d '"') '{"provide_liquidity": {"assets": [{"info": {"native_token": {"denom": "uscrt"}}, "amount": "100000000"}, {"info": {"token": {"contract_addr": '$token_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}, "amount": "1000000000"}]}}' --amount 100000000uscrt --from $deployer_name -y --gas 1500000 -b block

secretcli tx compute execute $(echo "$lptoken" | tr -d '"') '{"set_viewing_key": {"key": "yo"}}' -b block -y --from $deployer_name

lpbalance=$(secretcli q compute query $(echo "$lptoken" | tr -d '"') '{"balance": {"address": "'$deployer_address'", "key": "yo"}}' | jq '.balance.amount')
echo "LP Token balance: '$lpbalance'"

echo $(secretcli q compute query $(echo "$pair_contract" | tr -d '"') '{"simulation": {"offer_asset": {"info": {"native_token": {"denom": "uscrt"}}, "amount": "1000"}}}')


secretcli tx compute execute $(echo "$token_addr" | tr -d '"') '{"set_viewing_key": {"key": "yo"}}' -b block -y --from $deployer_name

tbalance=$(secretcli q compute query $(echo "$token_addr" | tr -d '"') '{"balance": {"address": "'$deployer_address'", "key": "yo"}}' | jq '.balance.amount')
echo "Token balance before swap: '$tbalance'"

balance=$(secretcli q account $deployer_address | jq '.value.coins[0].amount')

echo "USCRT balance before swap: '$balance'"

export TX_HASH=$(
  secretcli tx compute execute $(echo "$pair_contract" | tr -d '"') '{"swap": {"offer_asset": {"info": {"native_token": {"denom": "uscrt"}}, "amount": "1000"}}}' --amount 1000uscrt -b block -y --from $deployer_name --gas 1500000 |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

tbalance=$(secretcli q compute query $(echo "$token_addr" | tr -d '"') '{"balance": {"address": "'$deployer_address'", "key": "yo"}}' | jq '.balance.amount')
echo "Token balance after swap: '$tbalance'"
balance=$(secretcli q account $deployer_address | jq '.value.coins[0].amount')
echo "USCRT balance after swap: '$balance'"


export TX_HASH=$(
  secretcli tx compute execute $(echo "$token_addr" | tr -d '"') "{\"send\": {\"recipient\": $pair_contract,\"amount\":\"1000\",\"msg\":\"eyJzd2FwIjp7fX0K\"}}" -b block -y --from $deployer_name --gas 1500000 |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

tbalance=$(secretcli q compute query $(echo "$token_addr" | tr -d '"') '{"balance": {"address": "'$deployer_address'", "key": "yo"}}' | jq '.balance.amount')
echo "Token balance after swap: '$tbalance'"
balance=$(secretcli q account $deployer_address | jq '.value.coins[0].amount')
echo "USCRT balance after swap: '$balance'"


label=$(date +"%T")

export TX_HASH=$(
  secretcli tx compute instantiate $token_code_id '{"admin": "'$deployer_address'", "symbol": "TSTTST", "decimals": 6, "initial_balances": [{"address": "'$deployer_address'", "amount": "1000000000"}], "prng_seed": "YWE", "name": "test"}' --from $deployer_name --gas 1500000 --label "${label}" -b block -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

token2_addr=$(secretcli query compute list-contract-by-code $token_code_id | jq '.[-1].address')
echo "Token2 address: '$token2_addr'"

secretcli tx compute execute $(echo "$factory_contract" | tr -d '"') '{"create_pair": {"asset_infos": [{"native_token": {"denom": "uscrt"}},{"token": {"contract_addr": '$token2_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}]}}' --from $deployer_name -y --gas 1500000 -b block

secretcli q compute query $(echo "$factory_contract" | tr -d '"') '{"pairs": {}}'
secretcli q compute query $(echo "$factory_contract" | tr -d '"') '{"pairs": {"limit": 1}}'
secretcli q compute query $(echo "$factory_contract" | tr -d '"') '{"pairs": {"limit": 2}}'
secretcli q compute query $(echo "$factory_contract" | tr -d '"') '{"pairs": {"limit": 0}}'
secretcli q compute query $(echo "$factory_contract" | tr -d '"') '{"pairs": {"start_after":[{"native_token":{"denom":"uscrt"}},{"token":{"contract_addr":'$token2_addr',"token_code_hash":'$token_code_hash',"viewing_key":""}}]}}' # TODO fix: start_after doesn't work 

echo Factory: "$factory_contract" | tr -d '"'



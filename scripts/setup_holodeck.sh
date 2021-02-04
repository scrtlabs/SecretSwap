#!/bin/bash

set -xe

function wait_for_tx() {
  until (secretcli q tx "$1"); do
      sleep 5
  done
}

export deployer_name=yo
export wasm_path=build
export seth_addr='"secret1ttg5cn3mv5n9qv8r53stt6cjx8qft8ut9d66ed"'
export seth_hash=$(secretcli q compute contract $(echo "$seth_addr" | tr -d '"') | jq .code_id | parallel "secretcli q compute list-code | jq '.[] | select(.id == {})'" | jq .data_hash)
export sscrt_addr='"secret1s7c6xp9wltthk5r6mmavql4xld5me3g37guhsx"'
export sscrt_hash=$(secretcli q compute contract $(echo "$sscrt_addr" | tr -d '"') | jq .code_id | parallel "secretcli q compute list-code | jq '.[] | select(.id == {})'" | jq .data_hash)

export deployer_address=$(secretcli keys show -a $deployer_name)
echo "Deployer address: '$deployer_address'"

# store factory, pair & lp token contracts
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


# init factory
label="amm-${RANDOM}"
export TX_HASH=$(
  secretcli tx compute instantiate $factory_code_id '{"pair_code_id": '$pair_code_id', "pair_code_hash": '$pair_code_hash', "token_code_id": '$token_code_id', "token_code_hash": '$token_code_hash', "prng_seed": "YWE"}' --label $label --from $deployer_name -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

factory_contract=$(secretcli query compute list-contract-by-code $factory_code_id | jq '.[-1].address')
echo "Factory address: '$factory_contract'"

# create sscrt/seth pair
export TX_HASH=$(
  secretcli tx compute execute --label $label '{"create_pair": {"asset_infos": [{"token": {"contract_addr": '$sscrt_addr', "token_code_hash": '$sscrt_hash', "viewing_key": ""}},{"token": {"contract_addr": '$seth_addr', "token_code_hash": '$seth_hash', "viewing_key": ""}}]}}' --from $deployer_name -y --gas 1500000 -b block | jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

pair_contract=$(secretcli query compute list-contract-by-code $pair_code_id | jq '.[-1].address')
echo "Pair contract address: '$pair_contract'"

# provide 1000 seth / 100 sscrt
secretcli tx compute execute $(echo "$seth_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract', "amount": "1000000000000000000000"}}' -b block -y --from $deployer_name
secretcli tx compute execute $(echo "$sscrt_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract', "amount": "100000000"}}' -b block -y --from $deployer_name
export TX_HASH=$( 
  secretcli tx compute execute $(echo "$pair_contract" | tr -d '"') '{"provide_liquidity": {"assets": [{"info": {"token": {"contract_addr": '$sscrt_addr', "token_code_hash": '$sscrt_hash', "viewing_key": ""}}, "amount": "100000000"}, {"info": {"token": {"contract_addr": '$seth_addr', "token_code_hash": '$seth_hash', "viewing_key": ""}}, "amount": "1000000000000000000000"}]}}' --amount 100000000uscrt --from $deployer_name -y --gas 1500000 -b block | jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

echo Factory: "$factory_contract" | tr -d '"'

#!/bin/bash

set -xe

function wait_for_tx() {
  until (secretcli q tx "$1"); do
      sleep 5
  done
}

export deployer_name=enigma
export wasm_path=build
# export seth_addr='"secret1ttg5cn3mv5n9qv8r53stt6cjx8qft8ut9d66ed"'
# export seth_hash=$(secretcli q compute contract $(echo "$seth_addr" | tr -d '"') | jq .code_id | parallel "secretcli q compute list-code | jq '.[] | select(.id == {})'" | jq .data_hash)
# export sscrt_addr='"secret1s7c6xp9wltthk5r6mmavql4xld5me3g37guhsx"'
# export sscrt_hash=$(secretcli q compute contract $(echo "$sscrt_addr" | tr -d '"') | jq .code_id | parallel "secretcli q compute list-code | jq '.[] | select(.id == {})'" | jq .data_hash)

export deployer_address=$(secretcli keys show -a $deployer_name)
echo "Deployer address: '$deployer_address'"

# store factory, pair & lp token contracts
export TX_HASH=$(secretcli tx compute store "${wasm_path}/secretswap_token.wasm" --from "$deployer_name" --gas 2500000 -y | jq -r .txhash)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
token_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
token_code_hash=$(secretcli query compute list-code | jq '.[-1]."data_hash"')
echo "Stored token: '$token_code_id', '$token_code_hash'"

export TX_HASH=$(secretcli tx compute store "${wasm_path}/secretswap_factory.wasm" --from "$deployer_name" --gas 2500000 -y | jq -r .txhash)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
factory_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
echo "Stored factory: '$factory_code_id'"

export TX_HASH=$(secretcli tx compute store "${wasm_path}/secretswap_pair.wasm" --from "$deployer_name" --gas 2500000 -y | jq -r .txhash)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
pair_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
pair_code_hash=$(secretcli query compute list-code | jq '.[-1]."data_hash"')
echo "Stored pair: '$pair_code_id', '$pair_code_hash'"

# secretcli tx compute store "${wasm_path}/dummy_swap_data_receiver.wasm" --from "$deployer_name" --gas 3000000 -b block -y
# dummy_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
# dummy_code_hash=$(secretcli query compute list-code | jq '.[-1]."data_hash"')
# echo "Stored dummy: '$dummy_code_id', '$dummy_code_hash'"


# # init dummy cashback contract
# label="dummy-${RANDOM}"
# export TX_HASH=$(
#   secretcli tx compute instantiate $dummy_code_id '{}' --label $label --from $deployer_name -y |
#   jq -r .txhash
# )
# wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
# secretcli q compute tx $TX_HASH

# dummy_contract=$(secretcli query compute list-contract-by-code $dummy_code_id | jq '.[-1].address')
# echo "Dummy address: '$dummy_contract'"

# init factory
label="secretswap-factory"
export TX_HASH=$(
  secretcli tx compute instantiate $factory_code_id '{"pair_code_id": '$pair_code_id', "pair_code_hash": '$pair_code_hash', "token_code_id": '$token_code_id', "token_code_hash": '$token_code_hash', "prng_seed": "bG9sIG5vdCBpdA=="}' --label $label --from $deployer_name -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

factory_contract=$(secretcli query compute list-contract-by-code $factory_code_id | jq '.[-1].address')
echo "Factory address: '$factory_contract'"

# # create sscrt/seth pair
# export TX_HASH=$(
#   secretcli tx compute execute --label $label '{"create_pair": {"asset_infos": [{"token": {"contract_addr": '$sscrt_addr', "token_code_hash": '$sscrt_hash', "viewing_key": ""}},{"token": {"contract_addr": '$seth_addr', "token_code_hash": '$seth_hash', "viewing_key": ""}}]}}' --from $deployer_name -y --gas 1500000 | jq -r .txhash
# )
# wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
# secretcli q compute tx $TX_HASH

# pair_contract=$(secretcli query compute list-contract-by-code $pair_code_id | jq '.[-1].address')
# echo "Pair contract address: '$pair_contract'"

# # provide 1000 seth / 100 sscrt
# export TX_HASH=$(secretcli tx compute execute $(echo "$seth_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract', "amount": "1000000000000000000000"}}' -y --from $deployer_name | jq -r .txhash)
# wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
# secretcli q compute tx $TX_HASH

# export TX_HASH=$(secretcli tx compute execute $(echo "$sscrt_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract', "amount": "100000000"}}' -y --from $deployer_name | jq -r .txhash)
# wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
# secretcli q compute tx $TX_HASH

# export TX_HASH=$( 
#   secretcli tx compute execute $(echo "$pair_contract" | tr -d '"') '{"provide_liquidity": {"assets": [{"info": {"token": {"contract_addr": '$sscrt_addr', "token_code_hash": '$sscrt_hash', "viewing_key": ""}}, "amount": "100000000"}, {"info": {"token": {"contract_addr": '$seth_addr', "token_code_hash": '$seth_hash', "viewing_key": ""}}, "amount": "1000000000000000000000"}]}}' --amount 100000000uscrt --from $deployer_name -y --gas 1500000 | jq -r .txhash
# )
# wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
# secretcli q compute tx $TX_HASH

# # update factory with the dummy contract as a swap data endpoint
# secretcli tx compute execute $(echo "$factory_contract" | tr -d '"') '{"update_config": {"swap_data_endpoint": {"address":'$dummy_contract', "code_hash":'$dummy_code_hash'}}}' -b block -y --from $deployer_name

echo Factory: "$factory_contract" | tr -d '"'
# echo Dummy: "$dummy_contract" | tr -d '"'

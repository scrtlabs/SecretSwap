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

secretcli tx compute store "${wasm_path}/secretswap_token.wasm" --from "$deployer_name" --gas 3000000 -b block -y
token_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
token_code_hash=$(secretcli query compute list-code | jq '.[-1]."data_hash"')
echo "Stored token: '$token_code_id', '$token_code_hash'"

secretcli tx compute store "${wasm_path}/secretswap_factory.wasm" --from "$deployer_name" --gas 3000000 -b block -y
factory_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
echo "Stored factory: '$factory_code_id'"

secretcli tx compute store "${wasm_path}/secretswap_pair.wasm" --from "$deployer_name" --gas 3000000 -b block -y
pair_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
pair_code_hash=$(secretcli query compute list-code | jq '.[-1]."data_hash"')
echo "Stored pair: '$pair_code_id', '$pair_code_hash'"

secretcli tx compute store "${wasm_path}/secretswap_router.wasm" --from "$deployer_name" --gas 3000000 -b block -y
router_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
router_code_hash=$(secretcli query compute list-code | jq '.[-1]."data_hash"')
echo "Stored router: '$router_code_id', '$router_code_hash'"

secretcli tx compute store "${wasm_path}/dummy_swap_data_receiver.wasm" --from "$deployer_name" --gas 3000000 -b block -y
dummy_code_id=$(secretcli query compute list-code | jq '.[-1]."id"')
dummy_code_hash=$(secretcli query compute list-code | jq '.[-1]."data_hash"')
echo "Stored dummy: '$dummy_code_id', '$dummy_code_hash'"

echo "Deploying ETH..."

export TX_HASH=$(
  secretcli tx compute instantiate $token_code_id '{"admin": "'$deployer_address'", "symbol": "SETH", "decimals": 18, "initial_balances": [{"address": "'$deployer_address'", "amount": "100000000000000000000000"}], "prng_seed": "YWE=", "name": "test"}' --from $deployer_name --gas 1500000 --label ETH -b block -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

eth_addr=$(secretcli query compute list-contract-by-code $token_code_id | jq '.[-1].address')
echo "ETH address: '$eth_addr'"

echo "Deploying SCRT..."

export TX_HASH=$(
  secretcli tx compute instantiate $token_code_id '{"admin": "'$deployer_address'", "symbol": "SSCRT", "decimals": 6, "initial_balances": [{"address": "'$deployer_address'", "amount": "100000000000"}], "prng_seed": "YWE=", "name": "test"}' --from $deployer_name --gas 1500000 --label SSCRT -b block -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

scrt_addr=$(secretcli query compute list-contract-by-code $token_code_id | jq '.[-1].address')
echo "sSCRT address: '$scrt_addr'"

echo "Deploying SWBTC..."

export TX_HASH=$(
  secretcli tx compute instantiate $token_code_id '{"admin": "'$deployer_address'", "symbol": "SWBTC", "decimals": 8, "initial_balances": [{"address": "'$deployer_address'", "amount": "10000000000000"}], "prng_seed": "YWE=", "name": "test"}' --from $deployer_name --gas 1500000 --label SWBTC -b block -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

wbtc_addr=$(secretcli query compute list-contract-by-code $token_code_id | jq '.[-1].address')
echo "SWBTC address: '$wbtc_addr'"

echo "Deploying dummy cashback..."

export TX_HASH=$(
  secretcli tx compute instantiate $dummy_code_id '{}' --label dummy --from $deployer_name -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

dummy_contract=$(secretcli query compute list-contract-by-code $dummy_code_id | jq '.[-1].address')
echo "Dummy address: '$dummy_contract'"

echo "Deploying router..."

label=router
export TX_HASH=$(
  secretcli tx compute instantiate $router_code_id '{}' --label router --from $deployer_name -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

router_contract=$(secretcli query compute list-contract-by-code $router_code_id | jq '.[-1].address')
echo "Router address: '$router_contract'"

echo "Registering sETH,sSCRT,sWBTC in router..."
export TX_HASH=$(
  secretcli tx compute execute --label router '{"register_tokens":{"tokens":[{"address":'$scrt_addr',"code_hash":'$token_code_hash'},{"address":'$eth_addr',"code_hash":'$token_code_hash'},{"address":'$wbtc_addr',"code_hash":'$token_code_hash'}]}}' --from $deployer_name -y --gas 500000 |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

echo "Deploying AMM factory..."

export TX_HASH=$(
  secretcli tx compute instantiate $factory_code_id '{"pair_code_id": '$pair_code_id', "pair_code_hash": '$pair_code_hash', "token_code_id": '$token_code_id', "token_code_hash": '$token_code_hash', "prng_seed": "YWE="}' --label SecretSwap --from $deployer_name -y |
  jq -r .txhash
)
wait_for_tx "$TX_HASH" "Waiting for tx to finish on-chain..."
secretcli q compute tx $TX_HASH

factory_contract=$(secretcli query compute list-contract-by-code $factory_code_id | jq '.[-1].address')
echo "Factory address: '$factory_contract'"

echo "Creating sETH/sSCRT pair..."

secretcli tx compute execute --label SecretSwap '{"create_pair": {"asset_infos": [{"token": {"contract_addr": '$eth_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}},{"token": {"contract_addr": '$scrt_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}]}}' --from $deployer_name -y --gas 1500000 -b block

pair_contract_eth_sscrt=$(secretcli query compute list-contract-by-code $pair_code_id | jq '.[-1].address')
echo "sETH/sSCRT Pair contract address: '$pair_contract_eth_sscrt'"

secretcli tx compute execute $(echo "$eth_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract_eth_sscrt', "amount": "1000000000000000000000"}}' -b block -y --from $deployer_name
secretcli tx compute execute $(echo "$scrt_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract_eth_sscrt', "amount": "2000000000"}}' -b block -y --from $deployer_name
secretcli tx compute execute $(echo "$pair_contract_eth_sscrt" | tr -d '"') '{"provide_liquidity": {"assets": [{"info": {"token": {"contract_addr": '$scrt_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}, "amount": "2000000000"}, {"info": {"token": {"contract_addr": '$eth_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}, "amount": "1000000000000000000000"}]}}' --from $deployer_name -y --gas 1500000 -b block

echo "Creating sWBTC/sSCRT pair..."

secretcli tx compute execute --label SecretSwap '{"create_pair": {"asset_infos": [{"token": {"contract_addr": '$wbtc_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}},{"token": {"contract_addr": '$scrt_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}]}}' --from $deployer_name -y --gas 1500000 -b block

pair_contract_wbtc_sscrt=$(secretcli query compute list-contract-by-code $pair_code_id | jq '.[-1].address')
echo "sWBTC/sSCRT Pair contract address: '$pair_contract_wbtc_sscrt'"

secretcli tx compute execute $(echo "$wbtc_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract_wbtc_sscrt', "amount": "100000000000"}}' -b block -y --from $deployer_name
secretcli tx compute execute $(echo "$scrt_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract_wbtc_sscrt', "amount": "50000000000"}}' -b block -y --from $deployer_name
secretcli tx compute execute $(echo "$pair_contract_wbtc_sscrt" | tr -d '"') '{"provide_liquidity": {"assets": [{"info": {"token": {"contract_addr": '$scrt_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}, "amount": "50000000000"}, {"info": {"token": {"contract_addr": '$wbtc_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}, "amount": "100000000000"}]}}' --from $deployer_name -y --gas 1500000 -b block

echo "Creating SCRT/sSCRT pair..."

secretcli tx compute execute --label SecretSwap '{"create_pair": {"asset_infos": [{"native_token": {"denom": "uscrt"}},{"token": {"contract_addr": '$scrt_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}]}}' --from $deployer_name -y --gas 1500000 -b block

pair_contract_sscrt_scrt=$(secretcli query compute list-contract-by-code $pair_code_id | jq '.[-1].address')
echo "SCRT/sSCRT Pair contract address: '$pair_contract_sscrt_scrt'"

secretcli tx compute execute $(echo "$scrt_addr" | tr -d '"') '{"increase_allowance": {"spender": '$pair_contract_sscrt_scrt', "amount": "5000000000"}}' -b block -y --from $deployer_name
secretcli tx compute execute $(echo "$pair_contract_sscrt_scrt" | tr -d '"') '{"provide_liquidity": {"assets": [{"info": {"native_token": {"denom": "uscrt"}}, "amount": "5000000000"}, {"info": {"token": {"contract_addr": '$scrt_addr', "token_code_hash": '$token_code_hash', "viewing_key": ""}}, "amount": "5000000000"}]}}' --from $deployer_name --amount 5000000000uscrt -y --gas 1500000 -b block

secretcli tx compute execute $(echo "$factory_contract" | tr -d '"') '{"update_config": {"swap_data_endpoint": {"address":'$dummy_contract', "code_hash":'$dummy_code_hash'}}}' -b block -y --from $deployer_name

secretcli tx send a secret1x6my6xxxkladvsupcka7k092m50rdw8pk8dpq9 100000000uscrt -y -b block
secretcli tx compute execute $(echo "$eth_addr" | tr -d '"') '{"transfer":{"recipient":"secret1x6my6xxxkladvsupcka7k092m50rdw8pk8dpq9","amount":"1000000000000000000000"}}' --from a -y -b block
secretcli tx compute execute $(echo "$wbtc_addr" | tr -d '"') '{"transfer":{"recipient":"secret1x6my6xxxkladvsupcka7k092m50rdw8pk8dpq9","amount":"100000000000"}}' --from a -y -b block
secretcli tx compute execute $(echo "$scrt_addr" | tr -d '"') '{"transfer":{"recipient":"secret1x6my6xxxkladvsupcka7k092m50rdw8pk8dpq9","amount":"1000000000"}}' --from a -y -b block

echo Factory: "$factory_contract" | tr -d '"'
echo Dummy: "$dummy_contract" | tr -d '"'
echo Router: "$router_contract" | tr -d '"'
echo ETH: "$eth_addr" | tr -d '"'
echo SCRT: "$scrt_addr" | tr -d '"'
echo WBTC: "$wbtc_addr" | tr -d '"'

echo Pairs:
secretcli q compute query $(echo "$factory_contract" | tr -d '"') '{"pairs":{}}' | jq -c .pairs

secretcli tx compute store secretswap_token.wasm --from a --gas 3000000
secretcli tx compute store secretswap_factory.wasm --from a --gas 3000000
secretcli tx compute store secretswap_pair.wasm --from a --gas 3000000

secretcli tx compute instantiate 1 '{"admin": "secret1n6pr6ptec0px8gd8xvzkc0qj0d9j9yw37cvzdm", "symbol": "TST", "decimals": 6, "initial_balances": [{"address": "secret1n6pr6ptec0px8gd8xvzkc0qj0d9j9yw37cvzdm", "amount": "1000000000"}], "prng_seed": "YWE", "name": "test"}' --from a --gas 1500000 --label test2

secretcli tx compute instantiate 7 '{"pair_code_id": 6, "pair_code_hash": "555240FDDD74013A3A0B00D1332AFDD984DBF3D76FABCB3291B65694E34CE570", "token_code_id": 4, "token_code_hash": "0F2D5878209AADB774C31C53A9092CC7E3639D814BA1DFE3B612B04E5F4A5A2E", "prng_seed": "YWE"}' --label factory3 --from a

secretcli tx compute execute --label factory3 '{"create_pair": {"asset_infos": [{"native_token": {"denom": "uscrt"}},{"token": {"contract_addr": "secret1my3jvl6zs2n27648zngqrtw8pd23nrkrh0f7ax", "token_code_hash": "0F2D5878209AADB774C31C53A9092CC7E3639D814BA1DFE3B612B04E5F4A5A2E", "viewing_key": ""}}]}}' --from a --gas 1500000



# Secret-SCRT - Privacy coin backed by SCRT

This is a privacy token implementation on the Secret Network. It is backed by
the native coin of the network (SCRT) and has a fixed 1-to-1 exchange ratio
with it.

Version 1.0.0 of this contract is deployed to mainnet at the address
`secret1k0jntykt7e4g3y88ltc60czgjuqdy4c9e8fzek`. The deployed binary can be
reproduced by checking out the commit tagged `v1.0.0` of this repository and
running the command `make compile-optimized-reproducible`.
See [Verifying build](#verifying-build) for full instructions of how to
verify the authenticity of the deployed binary.

Usage is pretty simple - you deposit SCRT into the contract, and you get SSCRT 
(or Secret-SCRT), which you can then use with the ERC-20-like functionality that
the contract provides including: sending/receiving/allowance and withdrawing
back to SCRT. 

In terms of privacy the deposit & withdrawals are public, as they are
transactions on-chain. The rest of the functionality is private (so no one can
see if you send SSCRT and to whom, and receiving SSCRT is also hidden). 

## Usage examples:

Usage examples here assume `v1.0.3` of the CLI is installed.
Users using `v1.0.2` of the CLI can instead send raw compute transactions
and queries based on the schema that the contract expects.

For full documentation see:
```
secretcli tx snip20 --help
secretcli q snip20 --help
```

To deposit: ***(This is public)***
```
secretcli tx snip20 deposit sscrt --amount 1000000uscrt --from <account>
```

To redeem: ***(This is public)***
```
secretcli tx snip20 redeem sscrt <amount-to-redeem> --from <account>
```

To send SSCRT: ***(Only you will be able to see the parameters you send here)***
`amount-to-send` should just be an integer number equal to the amount of
`uscrt` to send.
```
secretcli tx snip20 transfer sscrt <recipient-address> <amount-to-send> --from <account>
```

To create your viewing key: 
```
secretcli tx snip20 create-viewing-key sscrt --from <account>
```
This transaction will be expensive, so set your gas limit to about 3M
with `--gas 3000000`. The key will start with the prefix `api_key_....`.

To check your balance: ***(Only you will be able to see the response)***
```
secretcli q snip20 balance sscrt <account-address> <viewing-key>
```

To view your transaction history:
```
secretcli q snip20 history sscrt <account-address> <viewing-key> [optional: page, default: 0] [optional: page_size, default: 10]
```

## Play with it on testnet

The deployed SSCRT contract address on the testnet is
`secret1umwqjum7f4zmp9alr2kpmq4y5j4hyxlam896r3` and label `sscrt`

## Troubleshooting 

All transactions are encrypted, so if you want to see the error returned by a
failed transaction, you need to use the command

```
secretcli q compute tx <TX_HASH>
```

## Notes on SNIP-20 compliance

The secret-secret contract is fully compatible with the
[SNIP20 specification](https://github.com/SecretFoundation/SNIPs/blob/master/SNIP-20.md),
but it does not implement all the specified functions. Namely, it omits burning
and minting of new coins, and all related functionality.

This contract maks the following decisions which the specification left open
for specific contracts to make:

* Messages should be padded to a multiple of 256 bytes by convention to maximize
  privacy.
* Addresses of secret-secret accounts are the same as of their respective secret
  account.
* The exchange ratio is fixed at 1-to-1.
* The total supply is never reported, but it will always equal the amount of
  SCRT locked in the contract, which can be seen in the explorer.

For more information about the various messages that the contract supports,
you can find all the message types under the file `src/msg.rs`.

## Verifying build

Given the address of a contract, you can query its code hash (sha256) by running:
```
secretcli q compute contract-hash <contract-address>
```

You can verify that this hash is correct by comparing it to the decompressed
contract binary.

To get the contract binary for a specific tag or commit and calculate its hash,
run:
```
git checkout <tag-or-commit>
make compile-optimized-reproducible
gunzip -c contract.wasm.gz >contract.wasm
sha256sum contract.wasm
```

Now compare the result with the hash returned by `secretcli`.
If you compiled the same code that was used to build the deployed binary,
they should match :)

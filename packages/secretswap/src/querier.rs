use cosmwasm_std::{
    to_binary, AllBalanceResponse, Api, BalanceResponse, BankQuery, Coin, Extern, HumanAddr,
    Querier, QueryRequest, StdError, StdResult, Storage, Uint128, WasmQuery,
};
use secret_toolkit::snip20::{balance_query, token_info_query};

use crate::asset::{Asset, AssetInfo, PairInfo};
use crate::msg::{FactoryQueryMsg, PairQueryMsg, ReverseSimulationResponse, SimulationResponse};

const BLOCK_SIZE: usize = 256;

pub fn query_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account_addr: &HumanAddr,
    denom: String,
) -> StdResult<Uint128> {
    // load price form the oracle
    let balance: BalanceResponse = deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: HumanAddr::from(account_addr),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

pub fn query_all_balances<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account_addr: &HumanAddr,
) -> StdResult<Vec<Coin>> {
    // load price form the oracle
    let all_balances: AllBalanceResponse =
        deps.querier
            .query(&QueryRequest::Bank(BankQuery::AllBalances {
                address: HumanAddr::from(account_addr),
            }))?;
    Ok(all_balances.amount)
}

pub fn query_token_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contract_addr: &HumanAddr,
    contract_hash: &String,
    account_addr: &HumanAddr,
    viewing_key: &String,
) -> StdResult<Uint128> {
    let msg = balance_query(
        &deps.querier,
        account_addr.clone(),
        viewing_key.clone(),
        BLOCK_SIZE,
        contract_hash.clone(),
        contract_addr.clone(),
    )?;

    Ok(msg.amount)
}

pub fn query_supply<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contract_addr: &HumanAddr,
    contract_hash: &String,
) -> StdResult<Uint128> {
    // load price form the oracle

    let token_info = token_info_query(
        &deps.querier,
        BLOCK_SIZE,
        contract_hash.clone(),
        contract_addr.clone(),
    )?;

    if token_info.total_supply.is_none() {
        return Err(StdError::generic_err(
            "Tried to query a token with unavailable supply",
        ));
    }

    Ok(token_info.total_supply.unwrap())
}

// #[inline]
// fn concat(namespace: &[u8], key: &[u8]) -> Vec<u8> {
//     let mut k = namespace.to_vec();
//     k.extend_from_slice(key);
//     k
// }

pub fn query_pair_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    factory_contract: &HumanAddr,
    factory_contract_hash: &String,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<PairInfo> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_contract.clone(),
        callback_code_hash: factory_contract_hash.clone(),
        msg: to_binary(&FactoryQueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        })?,
    }))
}

pub fn simulate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    pair_contract: &HumanAddr,
    pair_contract_hash: &String,
    offer_asset: &Asset,
) -> StdResult<SimulationResponse> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.clone(),
        callback_code_hash: pair_contract_hash.clone(),
        msg: to_binary(&PairQueryMsg::Simulation {
            offer_asset: offer_asset.clone(),
        })?,
    }))
}

pub fn reverse_simulate<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    pair_contract: &HumanAddr,
    pair_contract_hash: &String,
    ask_asset: &Asset,
) -> StdResult<ReverseSimulationResponse> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.clone(),
        callback_code_hash: pair_contract_hash.clone(),
        msg: to_binary(&PairQueryMsg::ReverseSimulation {
            ask_asset: ask_asset.clone(),
        })?,
    }))
}

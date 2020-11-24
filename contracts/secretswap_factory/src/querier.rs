use cosmwasm_std::{
    from_binary, to_binary, Api, Binary, Extern, HumanAddr, Querier, QueryRequest, StdResult,
    Storage, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use secretswap::PairInfo;

// copied from secretswap_pair.. todo: move it to secretswap common package
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsgPair {
    Pair {},
}

pub fn query_liquidity_token<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contract_addr: &HumanAddr,
    code_hash: &String,
) -> StdResult<HumanAddr> {
    // load price form the oracle
    let res: Binary = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        callback_code_hash: code_hash.clone(),
        contract_addr: contract_addr.clone(),
        msg: to_binary(&QueryMsgPair::Pair {})?,
    }))?;

    let pair_info: PairInfo = from_binary(&res)?;
    Ok(pair_info.liquidity_token)
}

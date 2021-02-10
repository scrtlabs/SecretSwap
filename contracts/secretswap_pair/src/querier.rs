use cosmwasm_std::{
    to_binary, Api, Extern, HumanAddr, Querier, QueryRequest, StdResult, Storage, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use secretswap::PairSettings;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryFactory {
    PairSettings {},
}

pub fn query_pair_settings<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contract_addr: &HumanAddr,
    code_hash: &String,
) -> StdResult<PairSettings> {
    // load price form the oracle
    let pair_settings: PairSettings =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            callback_code_hash: code_hash.clone(),
            contract_addr: contract_addr.clone(),
            msg: to_binary(&QueryFactory::PairSettings {})?,
        }))?;

    Ok(pair_settings)
}

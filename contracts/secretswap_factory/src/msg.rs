use cosmwasm_std::{Binary, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use secretswap::{AssetInfo, Fee, InitHook, PairInfo, PairSettings, SwapDataEndpoint};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// Pair contract code ID, which is used to
    pub pair_code_id: u64,
    pub token_code_id: u64,
    pub init_hook: Option<InitHook>,
    pub token_code_hash: String,
    pub pair_code_hash: String,
    pub prng_seed: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// UpdateConfig update relevant code IDs
    UpdateConfig {
        owner: Option<HumanAddr>,
        token_code_id: Option<u64>,
        pair_code_id: Option<u64>,
        pair_code_hash: Option<String>,
        token_code_hash: Option<String>,
        swap_fee: Option<Fee>,
        swap_data_endpoint: Option<SwapDataEndpoint>,
    },
    /// CreatePair instantiates pair contract
    CreatePair {
        /// Asset infos
        asset_infos: [AssetInfo; 2],
        /// Init hook for after works
        init_hook: Option<InitHook>,
    },
    /// Register is invoked from created pair contract after initialzation
    Register { asset_infos: [AssetInfo; 2] },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    PairSettings {},
    Pair {
        asset_infos: [AssetInfo; 2],
    },
    Pairs {
        start_after: Option<[AssetInfo; 2]>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
    pub pair_code_id: u64,
    pub token_code_id: u64,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairsResponse {
    pub pairs: Vec<PairInfo>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairsSettingsResponse {
    pub pair_settings: PairSettings,
}

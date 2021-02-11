use crate::Asset;
use cosmwasm_std::{to_binary, CosmosMsg, HumanAddr, Uint128, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapDataEndpoint {
    pub address: HumanAddr,
    pub code_hash: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Fee {
    pub commission_rate_nom: Uint128,
    pub commission_rate_denom: Uint128,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairSettings {
    pub swap_fee: Fee,
    pub swap_data_endpoint: Option<SwapDataEndpoint>,
}

impl SwapDataEndpoint {
    pub fn into_msg(self, asset_in: Asset, asset_out: Asset, account: HumanAddr) -> CosmosMsg {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address,
            callback_code_hash: self.code_hash,
            msg: to_binary(&SwapDataEndpointMsg::ReceiveSwapData {
                asset_in,
                asset_out,
                account,
            })?,
            send: vec![],
        })
    }
}

pub enum SwapDataEndpointMsg {
    ReceiveSwapData {
        asset_in: Asset,
        asset_out: Asset,
        account: HumanAddr,
    },
}

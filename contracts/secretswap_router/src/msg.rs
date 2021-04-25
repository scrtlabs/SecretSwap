use std::collections::VecDeque;

use crate::state::SecretContract;
use cosmwasm_std::{Binary, HumanAddr, Uint128};
use schemars::JsonSchema;
use secretswap::Asset;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub register_tokens: Option<Vec<Snip20Data>>,
    pub cashback: Option<SecretContract>,
    pub owner: Option<HumanAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Hop {
    pub from_token: Token,
    pub pair_address: HumanAddr,
    pub pair_code_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Route {
    pub hops: VecDeque<Hop>,
    pub expected_return: Option<Uint128>,
    pub to: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Snip20Data {
    pub address: HumanAddr,
    pub code_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Token {
    Snip20(Snip20Data),
    Scrt,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive {
        from: HumanAddr,
        msg: Option<Binary>,
        amount: Uint128,
    },
    FinalizeRoute {},
    RegisterTokens {
        tokens: Vec<Snip20Data>,
    },
    RecoverFunds {
        token: Token,
        amount: Uint128,
        to: HumanAddr,
        snip20_send_msg: Option<Binary>,
    },
    UpdateSettings {
        new_owner: Option<HumanAddr>,
        new_cashback: Option<SecretContract>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    SupportedTokens {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Snip20Swap {
    Swap {
        expected_return: Option<Uint128>,
        to: Option<HumanAddr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NativeSwap {
    Swap {
        offer_asset: Asset,
        expected_return: Option<Uint128>,
        to: Option<HumanAddr>,
    },
}

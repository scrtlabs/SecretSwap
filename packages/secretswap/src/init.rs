use cosmwasm_std::{Binary, HumanAddr, StdError, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::asset::AssetInfo;
use crate::hook::InitHook;

//use secret_toolkit::snip20::{MinterResponse};

/// TokenContract InitMsg
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Balance {
    pub amount: Uint128,
    pub address: HumanAddr,
}

// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
// #[serde(rename_all = "snake_case")]
// pub enum ResponseStatus {
//     Success,
//     Failure,
// }
//
// #[derive(Serialize, Deserialize, JsonSchema, Debug)]
// #[serde(rename_all = "snake_case")]
// pub  {
//     // Native
//     Mint {
//         status: ResponseStatus
//     }
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairInitMsg {
    /// Asset infos
    pub asset_infos: [AssetInfo; 2],
    /// Token contract code id for initialization
    pub token_code_id: u64,
    pub token_code_hash: String,
    /// Hook for post initalization
    pub init_hook: Option<InitHook>,
    pub prng_seed: Binary,
}

/// TokenContract InitMsg
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TokenInitMsg {
    pub name: String,
    pub admin: Option<HumanAddr>,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Option<Vec<Balance>>,
    pub prng_seed: Binary,
    pub init_hook: Option<InitHook>,
    pub config: Option<InitConfig>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Default, Debug)]
#[serde(rename_all = "snake_case")]
pub struct InitConfig {
    /// Indicates whether the total supply is public or should be kept secret.
    /// default: False
    pub public_total_supply: Option<bool>,
}

impl TokenInitMsg {
    // pub fn get_cap(&self) -> Option<Uint128> {
    //     self.mint.as_ref().and_then(|v| v.cap)
    // }
    pub fn new(
        name: String,
        admin: HumanAddr,
        symbol: String,
        decimals: u8,
        prng_seed: Binary,
        init_hook: InitHook,
    ) -> Self {
        Self {
            name,
            admin: Some(admin),
            symbol,
            decimals,
            initial_balances: None,
            prng_seed,
            init_hook: Some(init_hook),
            config: Some(InitConfig {
                public_total_supply: Some(true),
            }),
        }
    }
    pub fn validate(&self) -> StdResult<()> {
        // Check name, symbol, decimals
        if !is_valid_name(&self.name) {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        if !is_valid_symbol(&self.symbol) {
            return Err(StdError::generic_err(
                "Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}",
            ));
        }
        if self.decimals > 18 {
            return Err(StdError::generic_err("Decimals must not exceed 18"));
        }
        Ok(())
    }
}

fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 50 {
        return false;
    }
    true
}

fn is_valid_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < 3 || bytes.len() > 12 {
        return false;
    }
    for byte in bytes.iter() {
        if (*byte != 45) && (*byte < 65 || *byte > 90) && (*byte < 97 || *byte > 122) {
            return false;
        }
    }
    true
}

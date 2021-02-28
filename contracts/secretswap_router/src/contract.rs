use cosmwasm_std::{
    debug_print, from_binary, to_binary, Api, Binary, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, InitResponse, Querier, StdError, StdResult, Storage,
};
use secret_toolkit::snip20;

use crate::msg::{HandleMsg, InitMsg, QueryMsg, Route};

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Receive { from, msg, amount } => {
            if let Some(bin_msg) = msg {
                match from_binary(&bin_msg)? {
                    Route { hops } => {}
                    _ => {
                        return Err(StdError::generic_err(format!(
                            "data not in correct type: {:?}",
                            bin_msg
                        )))
                    }
                }
            } else {
                return Err(StdError::generic_err("data should be given"));
            }
        }
        HandleMsg::RegisterRootToken {
            token_address,
            token_code_hash,
        } => {
            // TODO add token_address to list of registered tokens
            return Ok(HandleResponse {
                messages: vec![snip20::register_receive_msg(
                    env.contract_code_hash.clone(),
                    None,
                    256,
                    token_code_hash.clone(),
                    token_address.clone(),
                )?],
                log: vec![],
                data: None,
            });
        }
    };
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
    _msg: QueryMsg,
) -> StdResult<Binary> {
    Ok(Binary(vec![]))
}

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
                    Route { hops, to } => {
                        // TODO
                        // 1. save the remaining route to state (e.g. if the route is X/Y -> Y/Z -> Z->W then save Y/Z -> Z/W to state)
                        // 2. send `amount` X to pair X/Y
                    }
                    _ => { /* TODO error  */ }
                }
            } else {
                // TODO
                // 1. load route from state (Y/Z -> Z/W)
                // 2. save the remaining route to state (Z/W)
                // 3. send `amount` Y to pair Y/Z

                // 1'. load route from state (Z/W)
                // 2'. this is the last hop so delete the entire route state
                // 3'. send `amount` Z to pair Z/W with recepient `to`
            }
        }
        HandleMsg::RegisterRootTokens { tokens } => {
            let mut msgs = vec![];

            for token in tokens {
                msgs.push(snip20::register_receive_msg(
                    env.contract_code_hash.clone(),
                    None,
                    256,
                    token.code_hash.clone(),
                    token.address.clone(),
                )?)
                // TODO add token_address to list of registered tokens
            }

            return Ok(HandleResponse {
                messages: msgs,
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

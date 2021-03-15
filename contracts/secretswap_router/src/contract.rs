use cosmwasm_std::{
    debug_print, from_binary, to_binary, Api, Binary, Coin, CosmosMsg, Env, Extern, HandleResponse,
    InitResponse, Querier, StdError, StdResult, Storage, WasmMsg,
};
use secret_toolkit::snip20;
use secretswap::{Asset, AssetInfo};
use HandleMsg::RecoverFunds;

use crate::{
    msg::{HandleMsg, Hop, InitMsg, NativeSwap, QueryMsg, Route, Snip20Swap},
    state::{
        delete_route_state, read_owner, read_route_state, read_tokens, store_owner,
        store_route_state, store_tokens, RouteState,
    },
};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    store_owner(&mut deps.storage, &env.message.sender)?;
    store_tokens(&mut deps.storage, &vec![])?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Receive {
            from: _,
            msg: Some(msg),
            amount,
        } => {
            // This is the first msg from the user, with the entire route details
            // 1. save the remaining route to state (e.g. if the route is X/Y -> Y/Z -> Z->W then save Y/Z -> Z/W to state)
            // 2. send `amount` X to pair X/Y
            // 3. call FinalizeRoute to make sure everything went ok, otherwise revert the tx

            let Route { hops, to } = from_binary(&msg)?;

            if hops.len() < 2 {
                return Err(StdError::generic_err("route must be at least 2 hops"));
            }

            for i in 1..(hops.len() - 1) {
                if hops[i].from_token.address == None
                    || hops[i].from_token.code_hash == None
                    || hops[i].from_token.native_denom != None
                {
                    return Err(StdError::generic_err(
                        "cannot route via uscrt. uscrt can only be route input token or output token.",
                    ));
                }
            }

            let first_hop: Hop = hops[0].clone();

            let received_from_token_of_first_hop =
                Some(env.message.sender) == first_hop.from_token.address;

            let mut received_scrt = false;
            let mut scrt_is_first_hop = false;
            if env.message.sent_funds.len() == 1 {
                received_scrt = env.message.sent_funds[0].amount == amount
                    && env.message.sent_funds[0].denom == "uscrt";
                scrt_is_first_hop = first_hop.from_token.native_denom == Some("uscrt".into());
            }

            if !received_from_token_of_first_hop && !(received_scrt && scrt_is_first_hop) {
                return Err(StdError::generic_err(
                    "route can only be initiated by sending here the token of the first hop",
                ));
            }

            let remaining_hops: Vec<Hop> = hops[1..].to_vec();

            store_route_state(
                &mut deps.storage,
                &RouteState {
                    is_done: false,
                    remaining_route: Route {
                        hops: remaining_hops,
                        to,
                    },
                },
            )?;

            let mut msgs = vec![];

            if let (Some(token_address), Some(token_code_hash)) =
                (first_hop.from_token.address, first_hop.from_token.code_hash)
            {
                // first hop is a snip20
                msgs.push(snip20::send_msg(
                    first_hop.pair_address,
                    amount,
                    // build swap msg for the next hop
                    Some(to_binary(&Snip20Swap::Swap {
                        // set expected_return to None because we don't care about slippage mid-route
                        expected_return: None,
                        // set the recepient of the swap to be this contract (the router)
                        to: Some(env.contract.address.clone()),
                    })?),
                    None,
                    256,
                    token_code_hash,
                    token_address,
                )?);
            } else if first_hop.from_token.native_denom == Some("uscrt".into()) {
                // first hop is SCRT
                msgs.push(
                        // build swap msg for the next hop                    CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: first_hop.pair_address,
                        callback_code_hash: first_hop.pair_code_hash,
                        msg: to_binary(&NativeSwap::Swap {
                            offer_asset: Asset {
                                amount,
                                info: AssetInfo::NativeToken {
                                    denom: "uscrt".into(),
                                },
                            },
                            // set expected_return to None because we don't care about slippage mid-route
                            expected_return: None,
                            // set the recepient of the swap to be this contract (the router)
                            to: Some(env.contract.address.clone()),
                        })?,
                        send: vec![Coin::new(amount.u128(), "uscrt")],
                    }),
                );
            } else {
                // shouldn't be here because we've tested this a few lines before
                return Err(StdError::generic_err(
                    "cannot build swap message for first hop",
                ));
            }

            msgs.push(
                // finalize the route at the end, to make sure the route was completed successfully
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.clone(),
                    callback_code_hash: env.contract_code_hash.clone(),
                    msg: to_binary(&HandleMsg::FinalizeRoute {})?,
                    send: vec![],
                }),
            );

            Ok(HandleResponse {
                messages: msgs,
                log: vec![],
                data: None,
            })
        }
        HandleMsg::Receive {
            from: _,
            msg: None,
            amount,
        } => {
            // This is a receive msg somewhere along the route
            // 1. load route from state (Y/Z -> Z/W)
            // 2. save the remaining route to state (Z/W)
            // 3. send `amount` Y to pair Y/Z

            // 1'. load route from state (Z/W)
            // 2'. this is the last hop so delete the entire route state
            // 3'. send `amount` Z to pair Z/W with recepient `to`
            match read_route_state(&deps.storage)? {
                Some(RouteState {
                    is_done: _,
                    remaining_route,
                }) => {
                    if remaining_route.hops.len() == 0 {
                        return Err(StdError::generic_err("route must be at least 1 hop"));
                    }

                    let next_hop: Hop = remaining_route.hops[0].clone();

                    if Some(env.message.sender) != next_hop.from_token.address {
                        return Err(StdError::generic_err(
                            "route can only be called by sending here the token of the next hop",
                        ));
                    }

                    let mut is_done = false;
                    let mut msgs = vec![];
                    if remaining_route.hops.len() == 1 {
                        // last hop
                        // 1. set is_done to true for FinalizeRoute
                        // 2. set expected_return for the final swap
                        // 3. set the recepient of the final swap to be the user
                        is_done = true;
                        msgs.push(snip20::send_msg(
                            next_hop.pair_address,
                            amount,
                            Some(to_binary(&Snip20Swap::Swap {
                                expected_return: next_hop.expected_return,
                                to: Some(remaining_route.to.clone()),
                            })?),
                            None,
                            256,
                            next_hop.from_token.code_hash.unwrap(), // tested before first hop
                            next_hop.from_token.address.unwrap(),   // tested before first hop
                        )?)
                    } else {
                        // not last hop
                        // 1. set expected_return to None because we don't care about slippage mid-route
                        // 2. set the recepient of the swap to be this contract (the router)
                        msgs.push(snip20::send_msg(
                            next_hop.pair_address,
                            amount,
                            Some(to_binary(&Snip20Swap::Swap {
                                expected_return: None,
                                to: Some(env.contract.address.clone()),
                            })?),
                            None,
                            256,
                            next_hop.from_token.code_hash.unwrap(), // tested before first hop
                            next_hop.from_token.address.unwrap(),   // tested before first hop
                        )?)
                    }

                    let remaining_hops: Vec<Hop> = remaining_route.hops[1..].to_vec();

                    store_route_state(
                        &mut deps.storage,
                        &RouteState {
                            is_done,
                            remaining_route: Route {
                                hops: remaining_hops,
                                to: remaining_route.to.clone(),
                            },
                        },
                    )?;

                    Ok(HandleResponse {
                        messages: msgs,
                        log: vec![],
                        data: None,
                    })
                }
                None => Err(StdError::generic_err("cannot find route")),
            }
        }
        HandleMsg::FinalizeRoute {} => match read_route_state(&deps.storage)? {
            Some(RouteState {
                is_done,
                remaining_route,
            }) => {
                // this function is called only by the route creation function
                // it is intended to always make sure that the route was completed successfully
                // otherwise - revert the transaction

                if env.contract.address != env.message.sender {
                    return Err(StdError::unauthorized());
                }
                if !is_done {
                    return Err(StdError::generic_err("cannot finalize: route is not done"));
                }
                if remaining_route.hops.len() != 0 {
                    return Err(StdError::generic_err(
                        "cannot finalize: route still has hops",
                    ));
                }

                delete_route_state(&mut deps.storage);

                Ok(HandleResponse::default())
            }
            None => Err(StdError::generic_err("no route to finalize")),
        },
        HandleMsg::RegisterTokens { tokens } => {
            let owner = read_owner(&deps.storage)?;
            if owner != env.message.sender {
                return Err(StdError::unauthorized());
            }

            let mut registered_tokens = read_tokens(&deps.storage)?;
            let mut msgs = vec![];

            for token in tokens {
                if token.address == None || token.code_hash == None {
                    return Err(StdError::generic_err(format!(
                        "token must have an 'address' and a 'code_hash': {:?}",
                        token
                    )));
                }

                let address = token.address.unwrap();
                let code_hash = token.code_hash.unwrap();

                if registered_tokens.contains(&address) {
                    continue;
                }
                registered_tokens.push(address.clone());

                msgs.push(snip20::register_receive_msg(
                    env.contract_code_hash.clone(),
                    None,
                    256,
                    code_hash.clone(),
                    address.clone(),
                )?);
                msgs.push(snip20::set_viewing_key_msg(
                    "SecretSwap Router".into(),
                    None,
                    256,
                    code_hash.clone(),
                    address.clone(),
                )?);
            }

            store_tokens(&mut deps.storage, &registered_tokens)?;

            Ok(HandleResponse {
                messages: msgs,
                log: vec![],
                data: None,
            })
        }
        RecoverFunds {
            token,
            amount,
            to,
            snip20_send_msg,
        } => {
            let owner = read_owner(&deps.storage)?;
            if owner != env.message.sender {
                return Err(StdError::unauthorized());
            }

            if token.address == None || token.code_hash == None {
                return Err(StdError::generic_err(format!(
                    "token must have an 'address' and a 'code_hash': {:?}",
                    token
                )));
            }

            Ok(HandleResponse {
                messages: vec![snip20::send_msg(
                    to,
                    amount,
                    snip20_send_msg,
                    None,
                    256,
                    token.code_hash.unwrap(),
                    token.address.unwrap(),
                )?],
                log: vec![],
                data: None,
            })
        }
        HandleMsg::ChangeOwner { new_owner } => {
            let current_owner = read_owner(&deps.storage)?;
            if current_owner != env.message.sender {
                return Err(StdError::unauthorized());
            }

            store_owner(&mut deps.storage, &new_owner)?;
            Ok(HandleResponse::default())
        }
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
    _msg: QueryMsg,
) -> StdResult<Binary> {
    match _msg {
        QueryMsg::SupportedTokens {} => {
            let tokens = read_tokens(&_deps.storage)?;
            Ok(to_binary(&tokens)?)
        }
    }
}

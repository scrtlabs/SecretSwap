use std::{
    ops::{Add, Mul, Sub},
    u128,
};

use cosmwasm_std::{
    debug_print, from_binary, log, to_binary, Api, Binary, CanonicalAddr, Coin, CosmosMsg, Decimal,
    Env, Extern, HandleResponse, HandleResult, HumanAddr, InitResponse, Querier, StdError,
    StdResult, Storage, Uint128, WasmMsg,
};
use primitive_types::U256;
//use ::{Cw20HandleMsg, Cw20ReceiveMsg, MinterResponse};
use secret_toolkit::snip20;

use secretswap::{
    query_supply, Asset, AssetInfo, AssetInfoRaw, Factory, InitHook, PairInfo, PairInfoRaw,
    PairInitMsg, TokenInitMsg,
};

use crate::math::{decimal_multiplication, decimal_subtraction, reverse_decimal};
use crate::msg::{
    Cw20HookMsg, HandleMsg, PoolResponse, QueryMsg, ReverseSimulationResponse, SimulationResponse,
};

use crate::querier::query_pair_settings;
use crate::state::{read_pair_info, store_pair_info};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: PairInitMsg,
) -> StdResult<InitResponse> {
    // create viewing key
    let assets_viewing_key = String::from("SecretSwap");

    let mut asset0 = msg.asset_infos[0].to_raw(&deps)?;
    let mut asset1 = msg.asset_infos[1].to_raw(&deps)?;

    /* append set viewing key messages and store viewing keys */
    let mut messages = vec![];
    match &msg.asset_infos[0] {
        AssetInfo::Token {
            contract_addr,
            token_code_hash,
            ..
        } => {
            messages.push(snip20::set_viewing_key_msg(
                assets_viewing_key.clone(),
                None,
                256,
                token_code_hash.clone(),
                contract_addr.clone(),
            )?);
            messages.push(snip20::register_receive_msg(
                env.contract_code_hash.clone(),
                None,
                256,
                token_code_hash.clone(),
                contract_addr.clone(),
            )?);
            asset0 = AssetInfoRaw::Token {
                contract_addr: deps.api.canonical_address(&contract_addr)?,
                token_code_hash: token_code_hash.clone(),
                viewing_key: assets_viewing_key.clone(),
            };
        }
        _ => {}
    }
    match &msg.asset_infos[1] {
        AssetInfo::Token {
            contract_addr,
            token_code_hash,
            ..
        } => {
            messages.push(snip20::set_viewing_key_msg(
                assets_viewing_key.clone(),
                None,
                256,
                token_code_hash.clone(),
                contract_addr.clone(),
            )?);
            messages.push(snip20::register_receive_msg(
                env.contract_code_hash.clone(),
                None,
                256,
                token_code_hash.clone(),
                contract_addr.clone(),
            )?);
            asset1 = AssetInfoRaw::Token {
                contract_addr: deps.api.canonical_address(&contract_addr)?,
                token_code_hash: token_code_hash.clone(),
                viewing_key: assets_viewing_key.clone(),
            };
        }
        _ => {}
    }

    // Create LP token
    messages.extend(vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
        code_id: msg.token_code_id,
        msg: to_binary(&TokenInitMsg::new(
            format!(
                "SecretSwap Liquidity Provider (LP) token for {}-{}",
                &msg.asset_infos[0], &msg.asset_infos[1]
            ),
            env.contract.address.clone(),
            "SWAP-LP".to_string(),
            6,
            msg.prng_seed,
            InitHook {
                msg: to_binary(&HandleMsg::PostInitialize {})?,
                contract_addr: env.contract.address.clone(),
                code_hash: env.contract_code_hash,
            },
        ))?,
        send: vec![],
        label: format!(
            "{}-{}-SecretSwap-LP-Token-{}",
            &msg.asset_infos[0],
            &msg.asset_infos[1],
            &env.contract.address.clone()
        ),
        callback_code_hash: msg.token_code_hash.clone(),
    })]);

    if let Some(hook) = msg.init_hook {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: hook.contract_addr.clone(),
            callback_code_hash: hook.code_hash.clone(),
            msg: hook.msg,
            send: vec![],
        }));

        let pair_info: &PairInfoRaw = &PairInfoRaw {
            contract_addr: deps.api.canonical_address(&env.contract.address)?,
            liquidity_token: CanonicalAddr::default(),
            token_code_hash: msg.token_code_hash,
            asset_infos: [asset0, asset1],
            asset0_volume: Uint128(0),
            asset1_volume: Uint128(0),
            factory: Factory {
                address: hook.contract_addr,
                code_hash: hook.code_hash,
            },
        };

        // create viewing keys

        store_pair_info(&mut deps.storage, &pair_info)?;
    } else {
        return Err(StdError::generic_err(
            "Must provide the factory as init hook",
        ));
    }

    Ok(InitResponse {
        messages,
        log: vec![log("status", "success")], // See https://github.com/CosmWasm/wasmd/pull/386
    })
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::Receive { amount, msg, from } => receive_cw20(deps, env, from, amount, msg),
        HandleMsg::PostInitialize {} => try_post_initialize(deps, env),
        HandleMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
        } => try_provide_liquidity(deps, env, assets, slippage_tolerance),
        HandleMsg::Swap {
            offer_asset,
            expected_return,
            belief_price,
            max_spread,
            to,
        } => {
            if !offer_asset.is_native_token() {
                return Err(StdError::unauthorized());
            }

            try_swap(
                deps,
                env.clone(),
                env.message.sender,
                offer_asset,
                expected_return,
                belief_price,
                max_spread,
                to,
            )
        }
    }
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    //todo: figure out if this is "from" or "sender"
    from: HumanAddr,
    amount: Uint128,
    msg: Option<Binary>,
) -> HandleResult {
    let contract_addr = env.message.sender.clone();
    if let Some(bin_msg) = msg {
        match from_binary(&bin_msg)? {
            Cw20HookMsg::Swap {
                expected_return,
                belief_price,
                max_spread,
                to,
            } => {
                // only asset contract can execute this message
                let mut authorized: bool = false;
                let config: PairInfoRaw = read_pair_info(&deps.storage)?;
                let pools: [Asset; 2] = config.query_pools(deps, &env.contract.address)?;
                for pool in pools.iter() {
                    if let AssetInfo::Token { contract_addr, .. } = &pool.info {
                        if contract_addr == &env.message.sender {
                            authorized = true;
                        }
                    }
                }

                if !authorized {
                    return Err(StdError::unauthorized());
                }

                try_swap(
                    deps,
                    env,
                    from,
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr,
                            token_code_hash: Default::default(),
                            viewing_key: Default::default(),
                        },
                        amount,
                    },
                    expected_return,
                    belief_price,
                    max_spread,
                    to,
                )
            }
            Cw20HookMsg::WithdrawLiquidity {} => {
                let config: PairInfoRaw = read_pair_info(&deps.storage)?;
                if deps.api.canonical_address(&env.message.sender)? != config.liquidity_token {
                    return Err(StdError::unauthorized());
                }

                try_withdraw_liquidity(deps, env, from, amount)
            }
        }
    } else {
        Err(StdError::generic_err("data should be given"))
    }
}

// Must token contract execute it
pub fn try_post_initialize<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let config: PairInfoRaw = read_pair_info(&deps.storage)?;

    // permission check
    if config.liquidity_token != CanonicalAddr::default() {
        return Err(StdError::unauthorized());
    }

    store_pair_info(
        &mut deps.storage,
        &PairInfoRaw {
            liquidity_token: deps.api.canonical_address(&env.message.sender)?,
            ..config.clone()
        },
    )?;

    Ok(HandleResponse {
        messages: vec![snip20::register_receive_msg(
            env.contract_code_hash,
            None,
            256,
            config.token_code_hash,
            env.message.sender.clone(),
        )?],
        log: vec![log("liquidity_token_addr", env.message.sender.as_str())],
        data: None,
    })
}

/// CONTRACT - should approve contract to use the amount of token
pub fn try_provide_liquidity<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
) -> HandleResult {
    for asset in assets.iter() {
        asset.assert_sent_native_token_balance(&env)?;
    }

    // Note: pair info + viewing keys are read from storage, therefore the input
    // viewing keys to this function are not used
    let pair_info: PairInfoRaw = read_pair_info(&deps.storage)?;
    let mut pools: [Asset; 2] = pair_info.query_pools(deps, &env.contract.address)?;
    let deposits: [Uint128; 2] = [
        assets
            .iter()
            .find(|a| a.info.equal(&pools[0].info))
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
        assets
            .iter()
            .find(|a| a.info.equal(&pools[1].info))
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
    ];

    let mut i = 0;
    let mut messages: Vec<CosmosMsg> = vec![];
    for pool in pools.iter_mut() {
        // If the pool is token contract, then we need to execute TransferFrom msg to receive funds
        if let AssetInfo::Token {
            contract_addr,
            token_code_hash,
            ..
        } = &pool.info
        {
            messages.push(snip20::transfer_from_msg(
                env.message.sender.clone(),
                env.contract.address.clone(),
                deposits[i],
                None,
                256,
                token_code_hash.clone(),
                contract_addr.clone(),
            )?);
        } else {
            // If the asset is native token, balance is already increased
            // To calculated properly we should subtract user deposit from the pool
            pool.amount = (pool.amount - deposits[i])?;
        }

        i += 1;
    }

    // assert slippage tolerance
    assert_slippage_tolerance(&slippage_tolerance, &deposits, &pools)?;

    let liquidity_token = deps.api.human_address(&pair_info.liquidity_token)?;
    let total_share = query_supply(&deps, &liquidity_token, &pair_info.token_code_hash)?;
    let share = if total_share == Uint128::zero() {
        // Initial share = collateral amount
        let deposit_0 = U256::from(deposits[0].u128());
        let deposit_1 = U256::from(deposits[1].u128());

        let sqrt = deposit_0
            .checked_mul(deposit_1)
            .and_then(|mul| u256_sqrt(mul))
            .ok_or_else(|| {
                StdError::generic_err(format!(
                    "Cannot calculate sqrt(deposit_0 {} * deposit_1 {})",
                    deposit_0, deposit_1
                ))
            })?;

        Uint128(sqrt.low_u128())
    } else {
        // min(1, 2)
        // 1. sqrt(deposit_0 * exchange_rate_0_to_1 * deposit_0) * (total_share / sqrt(pool_0 * pool_1))
        // == deposit_0 * total_share / pool_0
        // 2. sqrt(deposit_1 * exchange_rate_1_to_0 * deposit_1) * (total_share / sqrt(pool_1 * pool_1))
        // == deposit_1 * total_share / pool_1

        // This was:
        // std::cmp::min(
        //   deposits[0].multiply_ratio(total_share, pools[0].amount),
        //   deposits[1].multiply_ratio(total_share, pools[1].amount),
        // )

        let total_share = U256::from(total_share.u128());

        let deposit0 = U256::from(deposits[0].u128());
        let pools0_amount = U256::from(pools[0].amount.u128());

        let share0 = deposit0
            .checked_mul(total_share)
            .and_then(|res| res.checked_div(pools0_amount))
            .ok_or_else(|| {
                StdError::generic_err(format!(
                    "Cannot calculate deposits[0] {} * total_share {} / pools[0].amount {}",
                    deposit0, total_share, pools0_amount
                ))
            })?;

        let deposit1 = U256::from(deposits[1].u128());
        let pools1_amount = U256::from(pools[1].amount.u128());

        let share1 = deposit1
            .checked_mul(total_share)
            .and_then(|res| res.checked_div(pools1_amount))
            .ok_or_else(|| {
                StdError::generic_err(format!(
                    "Cannot calculate deposits[1] {} * total_share {} / pools[1].amount {}",
                    deposit1, total_share, pools1_amount
                ))
            })?;

        Uint128(std::cmp::min(share0, share1).low_u128())
    };

    messages.push(snip20::mint_msg(
        env.message.sender,
        share,
        None,
        256,
        pair_info.token_code_hash,
        deps.api.human_address(&pair_info.liquidity_token)?,
    )?);

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "provide_liquidity"),
            log("assets", format!("{}, {}", assets[0], assets[1])),
            log("share", &share),
        ],
        data: None,
    })
}

/// U256 sqrt ported from here: https://ethereum.stackexchange.com/a/87713/12112
///
/// function sqrt(uint y) internal pure returns (uint z) {
///     if (y > 3) {
///         z = y;
///         uint x = y / 2 + 1;
///         while (x < z) {
///             z = x;
///             x = (y / x + x) / 2;
///         }
///     } else if (y != 0) {
///         z = 1;
///     }
/// }
///
/// Tested it here: https://github.com/enigmampc/u256-sqrt-test/blob/aa7693/src/main.rs
fn u256_sqrt(y: U256) -> Option<U256> {
    let mut z = U256::from(0);
    if y.gt(&U256::from(3)) {
        z = y.clone();
        let mut x = y.checked_div(U256::from(2))?.checked_add(U256::from(1))?;
        while x.lt(&z) {
            z = x.clone();
            x = y
                .checked_div(x)?
                .checked_add(x)?
                .checked_div(U256::from(2))?;
        }
    } else if !y.is_zero() {
        z = U256::from(1);
    }

    return Some(z);
}

pub fn try_withdraw_liquidity<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    amount: Uint128,
) -> HandleResult {
    let pair_info: PairInfoRaw = read_pair_info(&deps.storage)?;
    let liquidity_addr: HumanAddr = deps.api.human_address(&pair_info.liquidity_token)?;

    let pools: [Asset; 2] = pair_info.query_pools(&deps, &env.contract.address)?;
    let total_share: Uint128 = query_supply(&deps, &liquidity_addr, &pair_info.token_code_hash)?;

    let refund_assets: Vec<Asset> = pools
        .iter()
        .map(|a| {
            // new_asset_amount = a.amount * amount / total_share

            let new_asset_amount = U256::from(a.amount.u128())
                .checked_mul(U256::from(amount.u128()))
                .and_then(|res| res.checked_div(U256::from(total_share.u128())))
                .ok_or_else(|| {
                    StdError::generic_err(format!(
                        "Cannot calculate a.amount {} * amount {} / total_share {}",
                        a.amount, amount, total_share
                    ))
                })?;

            Ok(Asset {
                info: a.info.clone(),
                amount: Uint128(new_asset_amount.low_u128()),
            })
        })
        .collect::<StdResult<Vec<Asset>>>()?;

    // update pool info
    Ok(HandleResponse {
        messages: vec![
            // refund asset tokens
            refund_assets[0].clone().into_msg(
                deps,
                env.contract.address.clone(),
                sender.clone(),
            )?,
            refund_assets[1].clone().into_msg(
                deps,
                env.contract.address.clone(),
                sender.clone(),
            )?,
            // burn liquidity token
            snip20::burn_msg(
                amount,
                None,
                256,
                pair_info.token_code_hash,
                deps.api.human_address(&pair_info.liquidity_token)?,
            )?,
        ],
        log: vec![
            log("action", "withdraw_liquidity"),
            log("withdrawn_share", &amount.to_string()),
            log(
                "refund_assets",
                format!("{}, {}", refund_assets[0].clone(), refund_assets[1].clone()),
            ),
        ],
        data: None,
    })
}

// CONTRACT - a user must do token approval
pub fn try_swap<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    offer_asset: Asset,
    expected_return: Option<Uint128>,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<HumanAddr>,
) -> HandleResult {
    offer_asset.assert_sent_native_token_balance(&env)?;

    let mut pair_info: PairInfoRaw = read_pair_info(&deps.storage)?;

    let pools: [Asset; 2] = pair_info.query_pools(&deps, &env.contract.address)?;

    let offer_pool: Asset;
    let ask_pool: Asset;

    // If the asset balance is already increased
    // To calculated properly we should subtract user deposit from the pool
    if offer_asset.info.equal(&pools[0].info) {
        let pool_amount = U256::from(pools[0].amount.u128());
        let offer_amount = U256::from(offer_asset.amount.u128());

        let amount = pool_amount.checked_sub(offer_amount).ok_or_else(|| {
            StdError::generic_err("offer_amount larger than pool_amount + offer_amount")
        })?;

        offer_pool = Asset {
            amount: Uint128(amount.low_u128()),
            info: pools[0].info.clone(),
        };
        ask_pool = pools[1].clone();

        pair_info.asset0_volume = pair_info.asset0_volume.add(offer_asset.amount);
    } else if offer_asset.info.equal(&pools[1].info) {
        let pool_amount = U256::from(pools[1].amount.u128());
        let offer_amount = U256::from(offer_asset.amount.u128());

        let amount = pool_amount.checked_sub(offer_amount).ok_or_else(|| {
            StdError::generic_err("offer_amount larger than pool_amount + offer_amount")
        })?;

        offer_pool = Asset {
            amount: Uint128(amount.low_u128()),
            info: pools[1].info.clone(),
        };
        ask_pool = pools[0].clone();

        pair_info.asset1_volume = pair_info.asset1_volume.add(offer_asset.amount);
    } else {
        return Err(StdError::generic_err("Wrong asset info is given"));
    }

    store_pair_info(&mut deps.storage, &pair_info)?;

    let pair_settings = query_pair_settings(
        &deps,
        &pair_info.factory.address,
        &pair_info.factory.code_hash,
    )?;

    let offer_amount = offer_asset.amount;
    let (return_amount, spread_amount, commission_amount) = compute_swap(
        offer_pool.amount,
        ask_pool.amount,
        offer_amount,
        pair_settings.swap_fee.commission_rate_nom,
        pair_settings.swap_fee.commission_rate_denom,
    )?;

    // check max spread limit if exist
    assert_max_spread(
        belief_price,
        max_spread,
        expected_return,
        offer_amount,
        return_amount,
        commission_amount,
        spread_amount,
    )?;

    let return_asset = Asset {
        info: ask_pool.info.clone(),
        amount: return_amount,
    };

    let mut messages = Vec::<CosmosMsg>::new();
    messages.push(return_asset.clone().into_msg(
        &deps,
        env.contract.address.clone(),
        to.clone().unwrap_or(sender.clone()),
    )?);

    if let Some(data_endpoint) = pair_settings.swap_data_endpoint {
        messages.push(data_endpoint.into_msg(
            offer_asset.clone(),
            Asset {
                info: return_asset.info,
                amount: return_amount + commission_amount,
            },
            to.unwrap_or(sender),
        )?);
    }

    // 1. send collateral token from the contract to a user
    // 2. send inactive commission to collector
    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "swap"),
            log("offer_asset", offer_asset.info.to_string()),
            log("ask_asset", ask_pool.info.to_string()),
            log("offer_amount", offer_amount.to_string()),
            log("return_amount", return_amount.to_string()),
            log("spread_amount", spread_amount.to_string()),
            log("commission_amount", commission_amount.to_string()),
        ],
        data: None,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Pair {} => to_binary(&query_pair_info(&deps)?),
        QueryMsg::Pool {} => to_binary(&query_pool(&deps)?),
        QueryMsg::Simulation { offer_asset } => to_binary(&query_simulation(&deps, offer_asset)?),
        QueryMsg::ReverseSimulation { ask_asset } => {
            to_binary(&query_reverse_simulation(&deps, ask_asset)?)
        }
    }
}

pub fn query_pair_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<PairInfo> {
    let pair_info: PairInfoRaw = read_pair_info(&deps.storage)?;
    pair_info.to_normal(&deps)
}

pub fn query_pool<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<PoolResponse> {
    let pair_info: PairInfoRaw = read_pair_info(&deps.storage)?;
    let contract_addr = deps.api.human_address(&pair_info.contract_addr)?;
    let assets: [Asset; 2] = pair_info.query_pools(&deps, &contract_addr)?;
    let total_share: Uint128 = query_supply(
        &deps,
        &deps.api.human_address(&pair_info.liquidity_token)?,
        &pair_info.token_code_hash,
    )?;

    let resp = PoolResponse {
        assets,
        total_share,
    };

    Ok(resp)
}

pub fn query_simulation<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    offer_asset: Asset,
) -> StdResult<SimulationResponse> {
    let pair_info: PairInfoRaw = read_pair_info(&deps.storage)?;

    let contract_addr = deps.api.human_address(&pair_info.contract_addr)?;
    let pools: [Asset; 2] = pair_info.query_pools(&deps, &contract_addr)?;

    let offer_pool: Asset;
    let ask_pool: Asset;
    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = pools[0].clone();
        ask_pool = pools[1].clone();
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = pools[1].clone();
        ask_pool = pools[0].clone();
    } else {
        return Err(StdError::generic_err(
            "Given offer asset is not belong to pairs",
        ));
    }

    let pair_settings = query_pair_settings(
        &deps,
        &pair_info.factory.address,
        &pair_info.factory.code_hash,
    )?;

    let (return_amount, spread_amount, commission_amount) = compute_swap(
        offer_pool.amount,
        ask_pool.amount,
        offer_asset.amount,
        pair_settings.swap_fee.commission_rate_nom,
        pair_settings.swap_fee.commission_rate_denom,
    )?;

    Ok(SimulationResponse {
        return_amount,
        spread_amount,
        commission_amount,
    })
}

pub fn query_reverse_simulation<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    ask_asset: Asset,
) -> StdResult<ReverseSimulationResponse> {
    let pair_info: PairInfoRaw = read_pair_info(&deps.storage)?;

    let contract_addr = deps.api.human_address(&pair_info.contract_addr)?;
    let pools: [Asset; 2] = pair_info.query_pools(&deps, &contract_addr)?;

    let offer_pool: Asset;
    let ask_pool: Asset;
    if ask_asset.info.equal(&pools[0].info) {
        ask_pool = pools[0].clone();
        offer_pool = pools[1].clone();
    } else if ask_asset.info.equal(&pools[1].info) {
        ask_pool = pools[1].clone();
        offer_pool = pools[0].clone();
    } else {
        return Err(StdError::generic_err(
            "Given ask asset is not blong to pairs",
        ));
    }

    let pair_settings = query_pair_settings(
        &deps,
        &pair_info.factory.address,
        &pair_info.factory.code_hash,
    )?;

    let (offer_amount, spread_amount, commission_amount) = compute_offer_amount(
        offer_pool.amount,
        ask_pool.amount,
        ask_asset.amount,
        pair_settings.swap_fee.commission_rate_nom.0,
        pair_settings.swap_fee.commission_rate_denom.0,
    )?;

    Ok(ReverseSimulationResponse {
        offer_amount,
        spread_amount,
        commission_amount,
    })
}

pub fn amount_of(coins: &[Coin], denom: String) -> Uint128 {
    match coins.iter().find(|x| x.denom == denom) {
        Some(coin) => coin.amount,
        None => Uint128::zero(),
    }
}

fn compute_swap(
    offer_pool: Uint128,
    ask_pool: Uint128,
    offer_amount: Uint128,
    commission_rate_nom: Uint128,
    commission_rate_denom: Uint128,
) -> StdResult<(Uint128, Uint128, Uint128)> {
    // offer => ask
    let offer_pool = U256::from(offer_pool.u128());
    let ask_pool = U256::from(ask_pool.u128());
    let offer_amount = U256::from(offer_amount.u128());

    // cp = offer_pool * ask_pool
    // ask_amount = (ask_pool - cp / (offer_pool + offer_amount)) * (1 - commission_rate)

    let cp = offer_pool.checked_mul(ask_pool).ok_or_else(|| {
        StdError::generic_err(format!(
            "Cannot calculate offer_pool {} * ask_pool {}",
            offer_pool, ask_pool
        ))
    })?;

    let new_offer_pool = offer_pool.checked_add(offer_amount).ok_or_else(|| {
        StdError::generic_err(format!(
            "Cannot calculate offer_pool {} + offer_amount {}",
            offer_pool, offer_amount
        ))
    })?;
    // ask_amount = (ask_pool - cp / new_offer_pool) * (1 - commission_rate)

    let cp_div_new_offer_pool = cp.checked_div(new_offer_pool).ok_or_else(|| {
        StdError::generic_err(format!(
            "Cannot calculate cp {} / new_offer_pool {}",
            cp, new_offer_pool
        ))
    })?;
    // ask_amount = (ask_pool - cp_div_new_offer_pool) * (1 - commission_rate)

    let return_amount = ask_pool.checked_sub(cp_div_new_offer_pool).ok_or_else(|| {
        StdError::generic_err(format!(
            "Cannot calculate ask_pool {} - cp_div_new_offer_pool {}",
            ask_pool, cp_div_new_offer_pool
        ))
    })?;
    // ask_amount = return_amount * (1 - commission_rate)

    // calculate spread & commission
    // spread = offer_amount * ask_pool / offer_pool - return_amount
    let offer_amount_mul_ask_pool = offer_amount.checked_mul(ask_pool).ok_or_else(|| {
        StdError::generic_err(format!(
            "Cannot calculate offer_amount {} * ask_pool {}",
            offer_amount, ask_pool
        ))
    })?;
    // spread = offer_amount_mul_ask_pool / offer_pool - return_amount

    let offer_amount_mul_ask_pool_div_offer_pool = offer_amount_mul_ask_pool
        .checked_div(offer_pool)
        .ok_or_else(|| {
            StdError::generic_err(format!(
                "Cannot calculate offer_amount_mul_ask_pool {} / offer_pool {}",
                offer_amount_mul_ask_pool, offer_pool
            ))
        })?;
    // spread = offer_amount_mul_ask_pool_div_offer_pool - return_amount

    let spread_amount = offer_amount_mul_ask_pool_div_offer_pool
        .checked_sub(return_amount)
        .ok_or_else(|| {
            StdError::generic_err(format!(
                "Cannot calculate offer_amount_mul_ask_pool_div_offer_pool {} - return_amount {}",
                offer_amount_mul_ask_pool_div_offer_pool, return_amount
            ))
        })?;

    // commission_amount = return_amount * COMMISSION_RATE_NOM / COMMISSION_RATE_DENOM
    let commission_amount_nom = return_amount
        .checked_mul(U256::from(commission_rate_nom.u128()))
        .ok_or_else(|| {
            StdError::generic_err(format!(
                "Cannot calculate return_amount {} * COMMISSION_RATE_NOM {}",
                return_amount, commission_rate_nom
            ))
        })?;
    // commission_amount = commission_amount_nom / COMMISSION_RATE_DENOM

    let commission_amount = commission_amount_nom
        .checked_div(U256::from(commission_rate_denom.u128()))
        .ok_or_else(|| {
            StdError::generic_err(format!(
                "Cannot calculate commission_amount_nom {} / COMMISSION_RATE_DENOM {}",
                commission_amount_nom, commission_rate_denom
            ))
        })?;

    // commission will be absorbed to pool
    let return_amount = return_amount
        .checked_sub(commission_amount)
        .ok_or_else(|| {
            StdError::generic_err(format!(
                "Cannot calculate return_amount {} - commission_amount {}",
                return_amount, commission_amount
            ))
        })?;

    Ok((
        Uint128(return_amount.low_u128()),
        Uint128(spread_amount.low_u128()),
        Uint128(commission_amount.low_u128()),
    ))
}

fn compute_offer_amount(
    offer_pool: Uint128,
    ask_pool: Uint128,
    ask_amount: Uint128,
    commission_rate_nom: u128,
    commission_rate_denom: u128,
) -> StdResult<(Uint128, Uint128, Uint128)> {
    // ask => offer
    // offer_amount = cp / (ask_pool - ask_amount / (1 - commission_rate)) - offer_pool
    let cp = Uint128(offer_pool.u128() * ask_pool.u128());
    let one_minus_commission = decimal_subtraction(
        Decimal::one(),
        Decimal::from_ratio(commission_rate_nom, commission_rate_denom),
    )?;

    let offer_amount: Uint128 = (cp.multiply_ratio(
        1u128,
        (ask_pool - ask_amount * reverse_decimal(one_minus_commission))?,
    ) - offer_pool)?;

    let before_commission_deduction = ask_amount * reverse_decimal(one_minus_commission);
    let spread_amount = (offer_amount * Decimal::from_ratio(ask_pool, offer_pool)
        - before_commission_deduction)
        .unwrap_or_else(|_| Uint128::zero());
    let commission_amount = before_commission_deduction
        * Decimal::from_ratio(commission_rate_nom, commission_rate_denom);
    Ok((offer_amount, spread_amount, commission_amount))
}

/// If `expected_return` is given, we check against `return_amount`
/// Else if `belief_price` and `max_spread` both are given,
/// we compute new spread else we just use terraswap
/// spread to check `max_spread`
pub fn assert_max_spread(
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    expected_return: Option<Uint128>,
    offer_amount: Uint128,
    return_amount: Uint128,
    commission_amount: Uint128,
    spread_amount: Uint128,
) -> StdResult<()> {
    if let Some(expected_return) = expected_return {
        if return_amount.lt(&expected_return) {
            return Err(StdError::generic_err(
                "Operation fell short of expected_return",
            ));
        }
    } else if let (Some(max_spread), Some(belief_price)) = (max_spread, belief_price) {
        let return_amount = return_amount + commission_amount;
        let expected_return = offer_amount.mul(reverse_decimal(belief_price));

        let spread_amount =
            (expected_return.sub(return_amount)).unwrap_or_else(|_| Uint128::zero());

        if return_amount.lt(&expected_return)
            && Decimal::from_ratio(spread_amount, expected_return).gt(&max_spread)
        {
            return Err(StdError::generic_err(
                "Operation exceeds max spread limit with belief_price",
            ));
        }
    } else if let Some(max_spread) = max_spread {
        let return_amount = return_amount + commission_amount;
        if Decimal::from_ratio(spread_amount, return_amount.add(spread_amount)).gt(&max_spread) {
            return Err(StdError::generic_err("Operation exceeds max spread limit"));
        }
    }

    Ok(())
}

fn assert_slippage_tolerance(
    slippage_tolerance: &Option<Decimal>,
    deposits: &[Uint128; 2],
    pools: &[Asset; 2],
) -> StdResult<()> {
    if let Some(slippage_tolerance) = *slippage_tolerance {
        let one_minus_slippage_tolerance = decimal_subtraction(Decimal::one(), slippage_tolerance)?;

        // Ensure each prices are not dropped as much as slippage tolerance rate
        if decimal_multiplication(
            Decimal::from_ratio(deposits[0], deposits[1]),
            one_minus_slippage_tolerance,
        ) > Decimal::from_ratio(pools[0].amount, pools[1].amount)
            || decimal_multiplication(
                Decimal::from_ratio(deposits[1], deposits[0]),
                one_minus_slippage_tolerance,
            ) > Decimal::from_ratio(pools[1].amount, pools[0].amount)
        {
            return Err(StdError::generic_err(
                "Operation exceeds max splippage tolerance",
            ));
        }
    }

    Ok(())
}

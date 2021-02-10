use cosmwasm_std::{
    Api, CanonicalAddr, Extern, HumanAddr, Querier, StdError, StdResult, Storage, Uint128,
};
use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};
use schemars::JsonSchema;
use secretswap::{AssetInfoRaw, PairInfo, PairInfoRaw};
use serde::{Deserialize, Serialize};
static KEY_CONFIG: &[u8] = b"config";
static KEY_PAIR_SETTINGS: &[u8] = b"pair_settings";
static PAIR_TRACKER: &[u8] = b"pair_tracker";
static PREFIX_PAIR_INFO: &[u8] = b"pair_info";
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CallableContract {
    pub address: HumanAddr,
    pub code_hash: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DevFund {
    pub address: HumanAddr,
    pub fee: Fee,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Fee {
    pub commission_rate_nom: Uint128,
    pub commission_rate_denom: Uint128,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub pair_code_id: u64,
    pub token_code_id: u64,
    pub token_code_hash: String,
    pub pair_code_hash: String,
    pub prng_seed: Vec<u8>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairSettings {
    pub swap_fee: Fee,
    pub dev_fund: Option<DevFund>,
    pub swap_data_endpoint: Option<CallableContract>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, JsonSchema)]
pub struct PairTracker(pub Vec<Vec<u8>>);
pub fn store_pair_tracker<S: Storage>(storage: &mut S, data: &PairTracker) -> StdResult<()> {
    Singleton::new(storage, PAIR_TRACKER).save(data)
}
pub fn read_pair_tracker<S: Storage>(storage: &S) -> StdResult<PairTracker> {
    ReadonlySingleton::new(storage, PAIR_TRACKER).load()
}
pub fn store_config<S: Storage>(storage: &mut S, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}
pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}
pub fn store_pair_settings<S: Storage>(storage: &mut S, data: &PairSettings) -> StdResult<()> {
    Singleton::new(storage, KEY_PAIR_SETTINGS).save(data)
}
pub fn read_pair_settings<S: Storage>(storage: &S) -> StdResult<PairSettings> {
    ReadonlySingleton::new(storage, KEY_PAIR_SETTINGS).load()
}
pub fn store_pair<S: Storage>(storage: &mut S, data: &PairInfoRaw) -> StdResult<()> {
    let mut asset_infos = data.asset_infos.clone().to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(&b.as_bytes()));
    let key = &[asset_infos[0].as_bytes(), asset_infos[1].as_bytes()].concat();
    let mut pair_bucket: Bucket<S, PairInfoRaw> = Bucket::new(PREFIX_PAIR_INFO, storage);
    pair_bucket.save(key, &data)?;

    let mut tracker = read_pair_tracker(storage).unwrap_or_default();
    let key_as_vec = key.to_vec();
    if !tracker.0.iter().any(|i| *i == key_as_vec) {
        // new pair
        tracker.0.push(key_as_vec);
        store_pair_tracker(storage, &tracker)
    } else {
        // pair already stored in pair_tracker
        Ok(())
    }
}
pub fn read_pair_by_key<S: Storage>(storage: &S, asset_infos: &[u8]) -> StdResult<PairInfoRaw> {
    let pair_bucket: ReadonlyBucket<S, PairInfoRaw> =
        ReadonlyBucket::new(PREFIX_PAIR_INFO, storage);
    match pair_bucket.load(asset_infos) {
        Ok(v) => Ok(v),
        Err(_e) => Err(StdError::generic_err("no pair data stored")),
    }
}
pub fn read_pair<S: Storage>(
    storage: &S,
    asset_infos: &[AssetInfoRaw; 2],
) -> StdResult<PairInfoRaw> {
    let mut asset_infos = asset_infos.clone().to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(&b.as_bytes()));
    let pair_bucket: ReadonlyBucket<S, PairInfoRaw> =
        ReadonlyBucket::new(PREFIX_PAIR_INFO, storage);
    match pair_bucket.load(&[asset_infos[0].as_bytes(), asset_infos[1].as_bytes()].concat()) {
        Ok(v) => Ok(v),
        Err(_e) => Err(StdError::generic_err("no pair data stored")),
    }
}
// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_pairs<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<[AssetInfoRaw; 2]>,
    limit: Option<u32>,
) -> StdResult<Vec<PairInfo>> {
    //return pair_bucket.load()
    let tracker = read_pair_tracker(&deps.storage)?;
    let mut iter = tracker.0.iter();
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    if let Some(start) = calc_range_start(start_after) {
        iter.position(|key| key == &start);
    };
    let mut pairs = vec![];
    for _ in 0..limit {
        let info = iter.next();
        if info.is_none() {
            break;
        }
        pairs.push(read_pair_by_key(&deps.storage, info.unwrap())?.to_normal(&deps)?)
    }
    Ok(pairs)
    //Ok(vec![])
    // todo: fix
    // pair_bucket
    //     .range(start.as_deref(), None, Order::Ascending)
    //     .take(limit)
    //     .map(|item| {
    //         let (_, v) = item?;
    //         v.to_normal(&deps)
    //     })
    //     .collect()
}
// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<[AssetInfoRaw; 2]>) -> Option<Vec<u8>> {
    start_after.map(|asset_infos| {
        let mut asset_infos = asset_infos.to_vec();
        asset_infos.sort_by(|a, b| a.as_bytes().cmp(&b.as_bytes()));
        let mut v = [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()]
            .concat()
            .as_slice()
            .to_vec();
        v.push(1);
        v
    })
}

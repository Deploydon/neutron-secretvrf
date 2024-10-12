//Base from https://github.com/writersblockchain/ibchooks-secretVRF/blob/master/consumer-side/src/contract.rs
//Modified for Neutron.
//https://github.com/Deploydon/neutron-secretvrf

#[cfg(not(feature = "library"))]
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{JOBCOUNT, RANDOM_OUTCOMES};

use cosmwasm_std::{
    entry_point, to_json_binary, Binary,
    Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, coin
};
use cw2::set_contract_version;
use neutron_sdk::{
    bindings::msg::{IbcFee, NeutronMsg},
    bindings::query::NeutronQuery,
    query::min_ibc_fee::query_min_ibc_fee,
    sudo::msg::{RequestPacketTimeoutHeight, SudoMsg},
    NeutronError, NeutronResult,
};

use sha2::{Sha256, Digest};
use base64::Engine;

const CONTRACT_NAME: &str = "deploydon.com:NeutronSCRTVRF";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const SECRET_VRF_CONTRACT_ADDRESS: &str = "secret1up0mymn4993hgn7zpzu4je34w0n5s7l0mem7rk";
const SECRET_VRF_VERIFICATION_KEY: &str = "BClOY6gcGjBCqeaFskrg0VIzptmyftgfY329GcZOvr3/eH/C4pJ4nH6ch6W/gjog8UErnEpIbMUOmElayUOxDas=";

//https://www.mintscan.io/secret/relayers
const SECRET_TRANSFER_CHANNEL_ID: &str = "channel-144";
const CHAIN_TRANSFER_CHANNEL_ID: &str = "channel-1551";
const FEE_DENOM: &str = "untrn";


//Random number range.
const MIN_NUMBER: u64 = 1;
const MAX_NUMBER: u64 = 1000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
     Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    JOBCOUNT.save(deps.storage, &0)?;
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    match msg {
        ExecuteMsg::RequestRandom {} => request_random(deps, env),
        ExecuteMsg::ReceiveRandom { job_id, randomness, signature} => receive_random(deps, env, job_id, randomness, signature)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetRandom { job_id } => query_randomness(deps, job_id),
    }
}

#[entry_point]
pub fn sudo(_deps: DepsMut, _env: Env, _msg: SudoMsg) -> StdResult<Response> {
    //needed for the feerefunder otherwise it throws an error
    Ok(Response::default())
}


pub fn query_randomness(deps: Deps, job_id: u64) -> StdResult<Binary> {
    let random_outcome = RANDOM_OUTCOMES.load(deps.storage, &job_id.to_string())?;
    to_json_binary(&random_outcome)
}

fn receive_random(deps: DepsMut<NeutronQuery>, _env: Env, job_id: String, randomness: String, signature: String) -> NeutronResult<Response<NeutronMsg>> {
    if RANDOM_OUTCOMES.may_load(deps.storage, &job_id)?.is_some() {
        return Err(NeutronError::Std(StdError::generic_err("This job ID has already been used")));
    }
   
    let mut hasher = Sha256::new();
    hasher.update([job_id.clone(), randomness.clone()].concat().as_bytes());
    let hash_result = hasher.finalize();

    let signature_correct = deps.api.secp256k1_verify(
        &hash_result,
        &base64::engine::general_purpose::STANDARD.decode(&signature).unwrap(),
        &base64::engine::general_purpose::STANDARD.decode(SECRET_VRF_VERIFICATION_KEY).unwrap()
    )
    .map_err(|err| StdError::generic_err(err.to_string()))?;
    if !signature_correct {
        return Err(NeutronError::Std(StdError::generic_err("Could not verify Secret VRF signature")));
    }

    //Uses the generated hash of job id + received randomness to create a random seed, to then dervive our random number.
    let random_seed = u64::from_be_bytes(hash_result[..8].try_into().unwrap());
    let random_number = MIN_NUMBER + (random_seed % (MAX_NUMBER - MIN_NUMBER + 1));
    RANDOM_OUTCOMES.save(deps.storage, &job_id, &random_number)?;
    Ok(Response::default().add_attribute("random", "successfull"))
}
fn request_random(deps: DepsMut<NeutronQuery>, env: Env) -> NeutronResult<Response<NeutronMsg>> {
    let mut job_id = JOBCOUNT.load(deps.storage)?;
    job_id += 1;
    JOBCOUNT.save(deps.storage, &job_id)?;

    //create the IBC Hook memo that will be execute by Secret Network 
    let ibc_callback_hook_memo = format!(
        "{{\"wasm\": {{\"contract\": \"{}\", \"msg\": {{\"request_random\": {{\"job_id\": \"{}\", \"num_words\": \"1\", \"callback_channel_id\": \"{}\", \"callback_to_address\": \"{}\", \"timeout_sec_from_now\": \"{}\"}}}}}}}}",
        SECRET_VRF_CONTRACT_ADDRESS, // Secret VRF Contract address on Secret Network
        job_id.to_string(), 
        SECRET_TRANSFER_CHANNEL_ID, // IBC Channel on the Secret Network side to send it back 
        env.contract.address,
        "900" //IBC callback timeout, here 900s = 15 min
    );

    let fee = min_ntrn_ibc_fee(query_min_ibc_fee(deps.as_ref())?.min_fee);
    let coin = coin(1, FEE_DENOM);
    let msg = NeutronMsg::IbcTransfer {
        source_port: "transfer".to_string(),
        source_channel: CHAIN_TRANSFER_CHANNEL_ID.to_string(),
        sender: env.contract.address.to_string(),
        receiver: SECRET_VRF_CONTRACT_ADDRESS.to_string(),
        token: coin,
        timeout_height: RequestPacketTimeoutHeight {
            revision_number: None,
            revision_height: None,
        },
        timeout_timestamp: env.block.time.plus_seconds(900).nanos(),
        fee: fee,
        memo: ibc_callback_hook_memo,
    };

    Ok(Response::new().add_message(msg))
}


//Neutron requires a IBC fee. This recieves from query_min_ibc_fee to set the bare minimum.
fn min_ntrn_ibc_fee(fee: IbcFee) -> IbcFee {
    IbcFee {
        recv_fee: fee.recv_fee,
        ack_fee: fee
            .ack_fee
            .into_iter()
            .filter(|a| a.denom == FEE_DENOM)
            .collect(),
        timeout_fee: fee
            .timeout_fee
            .into_iter()
            .filter(|a| a.denom == FEE_DENOM)
            .collect(),
    }
}
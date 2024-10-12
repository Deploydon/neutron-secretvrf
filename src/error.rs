use cosmwasm_std::StdError;
use thiserror::Error;
use neutron_sdk::NeutronError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Neutron(#[from] NeutronError),

    #[error("Received invalid randomness")]
    InvalidRandomness,
}

impl From<ContractError> for NeutronError {
    fn from(error: ContractError) -> Self {
        NeutronError::Std(StdError::generic_err(error.to_string()))
    }
}

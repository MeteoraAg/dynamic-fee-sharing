//! Error module includes error messages and codes of the program
use anchor_lang::prelude::*;

/// Error messages and codes of the program
#[error_code]
#[derive(PartialEq)]
pub enum FeeVaultError {
    #[msg("Math operation overflow")]
    MathOverflow,

    #[msg("Mint is not supported")]
    InvalidMint,

    #[msg("Fee vault parameters are invalid")]
    InvalidFeeVaultParameters,

    #[msg("Amount is zero")]
    AmountIsZero,

    #[msg("Invalid user index")]
    InvalidUserIndex,

    #[msg("Invalid user address")]
    InvalidUserAddress,

    #[msg("Exceeded number of users allowed")]
    ExceededUser,

    #[msg("Invalid fee vault")]
    InvalidFeeVault,

    #[msg("Invalid dammv2 pool")]
    InvalidDammv2Pool,

    #[msg("Invalid dammv2 pool")]
    InvalidDbcPool,

    #[msg("Invalid signer")]
    InvalidSigner,
}

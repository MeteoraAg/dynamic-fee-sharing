use anchor_lang::prelude::Pubkey;
use anchor_lang::Discriminator;

pub const MAX_USER: usize = 5;
pub const PRECISION_SCALE: u8 = 64;

pub mod seeds {
    pub const FEE_VAULT_PREFIX: &[u8] = b"fee_vault";
    pub const FEE_VAULT_AUTHORITY_PREFIX: &[u8] = b"fee_vault_authority";
    pub const TOKEN_VAULT_PREFIX: &[u8] = b"token_vault";
}

// (program_id, instruction, index_of_token_vault_account)
// TODO should find a way to avoid hardcoding index of token_vault_account
pub static WHITELISTED_ACTIONS: [(Pubkey, &[u8], usize); 7] = [
    // damm v2
    (
        damm_v2::ID,
        damm_v2::client::args::ClaimPositionFee::DISCRIMINATOR,
        4,
    ),
    (
        damm_v2::ID,
        damm_v2::client::args::ClaimReward::DISCRIMINATOR,
        5,
    ),
    // DBC
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::CreatorWithdrawSurplus::DISCRIMINATOR,
        3,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::ClaimCreatorTradingFee::DISCRIMINATOR,
        3,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::PartnerWithdrawSurplus::DISCRIMINATOR,
        3,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::ClaimTradingFee::DISCRIMINATOR,
        4,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::WithdrawMigrationFee::DISCRIMINATOR,
        3,
    ),
];

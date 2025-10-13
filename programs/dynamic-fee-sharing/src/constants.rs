use anchor_lang::prelude::Pubkey;
use anchor_lang::Discriminator;

pub const MAX_USER: usize = 5;
pub const PRECISION_SCALE: u8 = 64;

pub mod seeds {
    pub const FEE_VAULT_PREFIX: &[u8] = b"fee_vault";
    pub const FEE_VAULT_AUTHORITY_PREFIX: &[u8] = b"fee_vault_authority";
    pub const TOKEN_VAULT_PREFIX: &[u8] = b"token_vault";
}

pub static WHITELISTED_ACTIONS: [(Pubkey, &[u8]); 7] = [
    // damm v2
    (
        damm_v2::ID,
        damm_v2::client::args::ClaimPositionFee::DISCRIMINATOR,
    ),
    (
        damm_v2::ID,
        damm_v2::client::args::ClaimReward::DISCRIMINATOR,
    ),
    // DBC
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::CreatorWithdrawSurplus::DISCRIMINATOR,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::ClaimCreatorTradingFee::DISCRIMINATOR,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::PartnerWithdrawSurplus::DISCRIMINATOR,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::ClaimTradingFee::DISCRIMINATOR,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::WithdrawMigrationFee::DISCRIMINATOR,
    ),
];

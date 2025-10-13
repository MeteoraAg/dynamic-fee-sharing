use anchor_lang::Discriminator;
use anchor_lang::{prelude::Pubkey, pubkey};

pub const MAX_USER: usize = 5;
pub const PRECISION_SCALE: u8 = 64;

pub mod seeds {
    pub const FEE_VAULT_PREFIX: &[u8] = b"fee_vault";
    pub const FEE_VAULT_AUTHORITY_PREFIX: &[u8] = b"fee_vault_authority";
    pub const TOKEN_VAULT_PREFIX: &[u8] = b"token_vault";
}

pub const DAMM_V2: Pubkey = pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
pub const DBC: Pubkey = pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");

pub const WHITELISTED_ACTIONS: [(Pubkey, &[u8]); 7] = [
    // damm v2
    (
        DAMM_V2,
        damm_v2::client::args::ClaimPositionFee::DISCRIMINATOR,
    ),
    (DAMM_V2, damm_v2::client::args::ClaimReward::DISCRIMINATOR),
    // DBC
    (
        DBC,
        dynamic_bonding_curve::client::args::CreatorWithdrawSurplus::DISCRIMINATOR,
    ),
    (
        DBC,
        dynamic_bonding_curve::client::args::ClaimCreatorTradingFee::DISCRIMINATOR,
    ),
    (
        DBC,
        dynamic_bonding_curve::client::args::PartnerWithdrawSurplus::DISCRIMINATOR,
    ),
    (
        DBC,
        dynamic_bonding_curve::client::args::ClaimTradingFee::DISCRIMINATOR,
    ),
    (
        DBC,
        dynamic_bonding_curve::client::args::WithdrawMigrationFee::DISCRIMINATOR,
    ),
];

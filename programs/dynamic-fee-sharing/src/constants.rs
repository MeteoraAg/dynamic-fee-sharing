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
//
// only validate the token_vault_account for the FeeVault.token_mint
// for action with two tokens, other token is not validated by design
//
// TODO should find a way to avoid hardcoding index of token_vault_account
pub static WHITELISTED_ACTIONS: [(Pubkey, &[u8], usize); 9] = [
    // damm v2
    (
        damm_v2::ID,
        damm_v2::client::args::ClaimPositionFee::DISCRIMINATOR,
        4, // token_b_account
    ),
    (
        damm_v2::ID,
        damm_v2::client::args::ClaimReward::DISCRIMINATOR,
        5, // user_token_account
    ),
    // DBC
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::CreatorWithdrawSurplus::DISCRIMINATOR,
        3, // token_quote_account
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::ClaimCreatorTradingFee::DISCRIMINATOR,
        3, // token_b_account
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::ClaimCreatorTradingFee2::DISCRIMINATOR,
        3,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::PartnerWithdrawSurplus::DISCRIMINATOR,
        3, // token_quote_account
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::ClaimTradingFee::DISCRIMINATOR,
        4, // token_b_account
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::ClaimTradingFee2::DISCRIMINATOR,
        4,
    ),
    (
        dynamic_bonding_curve::ID,
        dynamic_bonding_curve::client::args::WithdrawMigrationFee::DISCRIMINATOR,
        3, // token_quote_account
    ),
];

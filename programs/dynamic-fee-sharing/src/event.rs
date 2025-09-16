use anchor_lang::prelude::*;

use crate::{state::FundingType, InitializeFeeVaultParameters};

#[event]
pub struct EvtInitializeFeeVault {
    pub fee_vault: Pubkey,
    pub token_mint: Pubkey,
    pub owner: Pubkey,
    pub base: Pubkey, // for fee vault pda
    pub params: InitializeFeeVaultParameters,
}

#[event]
pub struct EvtFundFee {
    pub funding_type: FundingType,
    pub fee_vault: Pubkey,
    pub funder: Pubkey,
    pub funded_amount: u64,
    pub fee_per_share: u128,
}

#[event]
pub struct EvtClaimFee {
    pub fee_vault: Pubkey,
    pub user: Pubkey,
    pub index: u8,
    pub claimed_fee: u64,
}

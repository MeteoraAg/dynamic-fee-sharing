use anchor_lang::prelude::*;

use crate::params::InitializeFeeVaultParameters;

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
    pub source_program: Pubkey,
    pub fee_vault: Pubkey,
    pub funded_amount: u64,
    pub fee_per_share: u128,
    pub payload: Vec<u8>,
}

#[event]
pub struct EvtClaimFee {
    pub fee_vault: Pubkey,
    pub user: Pubkey,
    pub index: u8,
    pub claimed_fee: u64,
}

#[event]
pub struct EvtAddUser {
    pub fee_vault: Pubkey,
    pub user: Pubkey,
    pub share: u32,
}

#[event]
pub struct EvtUpdateUserShare {
    pub fee_vault: Pubkey,
    pub user: Pubkey,
    pub old_share: u32,
    pub new_share: u32,
}

#[event]
pub struct EvtRemoveUser {
    pub fee_vault: Pubkey,
    pub user: Pubkey,
    pub unclaimed_fee_0: u64,
    pub unclaimed_fee_1: u64,
}

#[event]
pub struct EvtClaimUnclaimedFee {
    pub fee_vault: Pubkey,
    pub user: Pubkey,
    pub claimed_fee_0: u64,
    pub claimed_fee_1: u64,
}

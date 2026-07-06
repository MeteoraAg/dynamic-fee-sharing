use anchor_lang::prelude::*;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use static_assertions::const_assert_eq;

#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    IntoPrimitive,
    TryFromPrimitive,
    AnchorDeserialize,
    AnchorSerialize,
)]
pub enum VaultType {
    NonPdaAccount,
    PdaAccount,
}

#[zero_copy]
#[derive(InitSpace, Debug, Default)]
pub struct VaultHeader {
    pub owner: Pubkey,
    pub token_mint: Pubkey,
    pub token_vault: Pubkey,
    pub token_flag: u8, // indicate whether token is spl-token or token2022
    pub vault_type: u8,
    pub vault_bump: u8,
    pub padding_0: [u8; 13],
    pub total_share: u32,
    pub padding_1: [u8; 4],
    pub total_funded_fee: u64,
    pub fee_per_share: u128,
    pub base: Pubkey,
    pub padding: [u128; 4],
}
const_assert_eq!(VaultHeader::INIT_SPACE, 240);

impl VaultHeader {
    pub fn initialize(
        &mut self,
        owner: &Pubkey,
        token_flag: u8,
        token_mint: &Pubkey,
        token_vault: &Pubkey,
        base: &Pubkey,
        vault_bump: u8,
        vault_type: u8,
    ) {
        self.owner = *owner;
        self.token_flag = token_flag;
        self.token_mint = *token_mint;
        self.token_vault = *token_vault;
        self.base = *base;
        self.vault_bump = vault_bump;
        self.vault_type = vault_type;
    }
}

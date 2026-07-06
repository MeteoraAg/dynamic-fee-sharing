use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;

#[zero_copy]
#[derive(InitSpace, Debug, Default)]
pub struct DynamicVaultHeader {
    pub owner: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_1_vault: Pubkey,
    pub token_0_flag: u8, // indicate whether token 0 is spl-token or token2022
    pub token_1_flag: u8, // indicate whether token 1 is spl-token or token2022
    pub vault_type: u8,
    pub vault_bump: u8,
    pub total_share: u32,
    pub total_funded_fee_token_0: u64,
    pub total_funded_fee_token_1: u64,
    pub padding_0: [u8; 8],
    pub fee_per_share_token_0: u128,
    pub fee_per_share_token_1: u128,
    pub base: Pubkey,
    pub padding: [u128; 4],
}
const_assert_eq!(DynamicVaultHeader::INIT_SPACE, 320);

impl DynamicVaultHeader {
    pub fn initialize(
        &mut self,
        owner: &Pubkey,
        token_0_flag: u8,
        token_0_mint: &Pubkey,
        token_0_vault: &Pubkey,
        token_1_flag: u8,
        token_1_mint: &Pubkey,
        token_1_vault: &Pubkey,
        base: &Pubkey,
        vault_bump: u8,
        vault_type: u8,
    ) {
        self.owner = *owner;
        self.token_0_flag = token_0_flag;
        self.token_0_mint = *token_0_mint;
        self.token_0_vault = *token_0_vault;
        self.token_1_flag = token_1_flag;
        self.token_1_mint = *token_1_mint;
        self.token_1_vault = *token_1_vault;
        self.base = *base;
        self.vault_bump = vault_bump;
        self.vault_type = vault_type;
    }
}

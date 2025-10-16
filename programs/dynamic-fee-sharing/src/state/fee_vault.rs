use crate::{
    constants::{MAX_USER, PRECISION_SCALE},
    error::FeeVaultError,
    instructions::UserShare,
    math::{mul_shr, shl_div, SafeMath},
};
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
pub enum FeeVaultType {
    NonPdaAccount,
    PdaAccount,
}

#[account(zero_copy)]
#[derive(InitSpace, Debug, Default)]
pub struct FeeVault {
    pub owner: Pubkey,
    pub token_mint: Pubkey,
    pub token_vault: Pubkey,
    pub token_flag: u8, // indicate whether token is spl-token or token2022
    pub fee_vault_type: u8,
    pub fee_vault_bump: u8,
    pub padding_0: [u8; 13],
    pub total_share: u32,
    pub padding_1: [u8; 4],
    pub total_funded_fee: u64,
    pub fee_per_share: u128,
    pub base: Pubkey,
    pub padding: [u128; 4],
    pub users: [UserFee; MAX_USER],
}
const_assert_eq!(FeeVault::INIT_SPACE, 640);

#[zero_copy]
#[derive(InitSpace, Debug, Default)]
pub struct UserFee {
    pub address: Pubkey,
    pub share: u32,
    pub padding_0: [u8; 4],
    pub fee_claimed: u64,
    pub padding: [u8; 16], // padding for future use
    pub fee_per_share_checkpoint: u128,
}
const_assert_eq!(UserFee::INIT_SPACE, 80);

impl FeeVault {
    pub fn initialize(
        &mut self,
        owner: &Pubkey,
        token_flag: u8,
        token_mint: &Pubkey,
        token_vault: &Pubkey,
        base: &Pubkey,
        fee_vault_bump: u8,
        fee_vault_type: u8,
        users: &[UserShare],
    ) -> Result<()> {
        self.owner = *owner;
        self.token_flag = token_flag;
        self.token_mint = *token_mint;
        self.token_vault = *token_vault;
        let mut total_share = 0;
        for i in 0..users.len() {
            self.users[i] = UserFee {
                address: users[i].address,
                share: users[i].share,
                ..Default::default()
            };
            total_share = total_share.safe_add(users[i].share)?;
        }
        self.total_share = total_share;
        self.base = *base;
        self.fee_vault_bump = fee_vault_bump;
        self.fee_vault_type = fee_vault_type;

        Ok(())
    }

    pub fn fund_fee(&mut self, amount: u64) -> Result<()> {
        self.total_funded_fee = self.total_funded_fee.safe_add(amount)?;

        let fee_per_share = shl_div(amount, self.total_share.into(), PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        self.fee_per_share = self.fee_per_share.safe_add(fee_per_share)?;

        Ok(())
    }

    pub fn validate_and_claim_fee(&mut self, index: u8, signer: &Pubkey) -> Result<u64> {
        let user = self
            .users
            .get_mut(index as usize)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(user.address.eq(signer), FeeVaultError::InvalidUserAddress);

        let reward_per_share_delta = self.fee_per_share.safe_sub(user.fee_per_share_checkpoint)?;

        let fee_being_claimed = mul_shr(user.share.into(), reward_per_share_delta, PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?
            .try_into()
            .map_err(|_| FeeVaultError::MathOverflow)?;

        user.fee_per_share_checkpoint = self.fee_per_share;
        user.fee_claimed = user.fee_claimed.safe_add(fee_being_claimed)?;

        Ok(fee_being_claimed)
    }

    pub fn is_share_holder(&self, signer: &Pubkey) -> bool {
        self.users
            .iter()
            .any(|share_holder| share_holder.address.eq(signer))
    }
}

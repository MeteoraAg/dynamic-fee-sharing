use crate::{
    constants::{MAX_STATIC_USER, PRECISION_SCALE},
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
    pub mutable_flag: u8, // indicate whether the fee vault is mutable by admin or operator, 0 or 1 only
    pub padding_0: [u8; 12],
    pub total_share: u32,
    pub padding_1: [u8; 4],
    pub total_funded_fee: u64,
    pub fee_per_share: u128,
    pub base: Pubkey,
    pub operator: Pubkey, // operator is the account that can update a mutable fee vault. default: owner
    pub padding: [u128; 2],
    pub users: [UserFee; MAX_STATIC_USER],
}
const_assert_eq!(FeeVault::INIT_SPACE, 640);

#[zero_copy]
#[derive(InitSpace, Debug, Default)]
pub struct UserFee {
    pub address: Pubkey,
    pub share: u32,
    pub padding_0: [u8; 4],
    pub fee_claimed: u64,
    pub pending_fee: u64,
    pub padding: [u8; 8], // padding for future use
    pub fee_per_share_checkpoint: u128,
}
const_assert_eq!(UserFee::INIT_SPACE, 80);

impl UserFee {
    pub fn new(address: Pubkey, share: u32, fee_per_share_checkpoint: u128) -> Self {
        Self {
            address,
            share,
            fee_per_share_checkpoint,
            ..Default::default()
        }
    }

    pub fn get_total_pending_fee(&self, fee_per_share: u128) -> Result<u64> {
        let delta = fee_per_share.safe_sub(self.fee_per_share_checkpoint)?;
        let current_pending_fee = mul_shr(self.share.into(), delta, PRECISION_SCALE)
            .and_then(|fee| fee.try_into().ok())
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        let total_pending_fee = self.pending_fee.safe_add(current_pending_fee)?;
        Ok(total_pending_fee)
    }
}

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
        mutable_flag: u8,
    ) -> Result<()> {
        self.owner = *owner;
        self.token_flag = token_flag;
        self.token_mint = *token_mint;
        self.token_vault = *token_vault;
        let mut total_share = 0;
        for (i, user) in users.iter().enumerate() {
            self.users[i] = UserFee::new(user.address, user.share, 0);
            total_share = total_share.safe_add(user.share)?;
        }
        self.total_share = total_share;
        self.base = *base;
        self.fee_vault_bump = fee_vault_bump;
        self.fee_vault_type = fee_vault_type;
        self.operator = *owner;
        self.mutable_flag = mutable_flag;

        Ok(())
    }

    pub fn fund_fee(&mut self, amount: u64) -> Result<()> {
        self.total_funded_fee = self.total_funded_fee.safe_add(amount)?;

        let fee_per_share = shl_div(amount, self.total_share.into(), PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        self.fee_per_share = self.fee_per_share.safe_add(fee_per_share)?;

        Ok(())
    }
}

use crate::{
    constants::{MAX_USER, MIN_USER, PRECISION_SCALE},
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
    pub operator: Pubkey,
    pub padding: [u128; 2],
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
    pub pending_fee: u64,
    pub padding: [u8; 8], // padding for future use
    pub fee_per_share_checkpoint: u128,
}
const_assert_eq!(UserFee::INIT_SPACE, 80);

impl UserFee {
    pub fn get_total_pending_fee(&self, fee_per_share: u128) -> Result<u64> {
        let delta = fee_per_share.safe_sub(self.fee_per_share_checkpoint)?;
        let current_pending_fee = mul_shr(self.share.into(), delta, PRECISION_SCALE)
            .and_then(|fee| fee.try_into().ok())
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        let total_pending_fee = self.pending_fee.safe_add(current_pending_fee)?;
        Ok(total_pending_fee)
    }
}

#[account(zero_copy)]
#[derive(InitSpace, Debug, Default)]
pub struct UserUnclaimedFee {
    pub unclaimed_fee: u64,
    pub padding: [u8; 32], //  padding for future use
}

const_assert_eq!(UserUnclaimedFee::INIT_SPACE, 40);

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
        self.operator = Pubkey::default();

        Ok(())
    }

    pub fn fund_fee(&mut self, amount: u64) -> Result<()> {
        self.total_funded_fee = self.total_funded_fee.safe_add(amount)?;

        let fee_per_share = shl_div(amount, self.total_share.into(), PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        self.fee_per_share = self.fee_per_share.safe_add(fee_per_share)?;

        Ok(())
    }

    pub fn validate_and_claim_fee(&mut self, index: usize, signer: &Pubkey) -> Result<u64> {
        let user = self
            .users
            .get_mut(index)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(user.address.eq(signer), FeeVaultError::InvalidUserAddress);

        let fee_being_claimed = user.get_total_pending_fee(self.fee_per_share)?;

        user.pending_fee = 0;
        user.fee_per_share_checkpoint = self.fee_per_share;
        user.fee_claimed = user.fee_claimed.safe_add(fee_being_claimed)?;

        Ok(fee_being_claimed)
    }

    pub fn is_share_holder(&self, signer: &Pubkey) -> bool {
        self.users
            .iter()
            .any(|share_holder| share_holder.address.eq(signer))
    }

    pub fn validate_and_update_share(
        &mut self,
        index: usize,
        user_address: &Pubkey,
        share: u32,
    ) -> Result<()> {
        let user = self
            .users
            .get_mut(index)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(
            user.address.eq(user_address) && user_address.ne(&Pubkey::default()),
            FeeVaultError::InvalidUserAddress
        );
        require!(
            share > 0 && share != user.share,
            FeeVaultError::InvalidFeeVaultParameters
        );

        self.total_share = self.total_share.safe_sub(user.share)?.safe_add(share)?;

        user.pending_fee = user.get_total_pending_fee(self.fee_per_share)?;
        user.fee_per_share_checkpoint = self.fee_per_share;
        user.share = share;

        Ok(())
    }

    pub fn validate_and_add_user(&mut self, user_address: &Pubkey, share: u32) -> Result<()> {
        // prevent adding duplicate user
        require!(
            user_address.ne(&Pubkey::default()) && !self.is_share_holder(user_address),
            FeeVaultError::InvalidUserAddress
        );

        require!(share > 0, FeeVaultError::InvalidFeeVaultParameters);

        let empty_slot = self
            .users
            .iter()
            .position(|user| user.address.eq(&Pubkey::default()))
            .ok_or_else(|| FeeVaultError::InvalidNumberOfUsers)?; // already full

        self.users[empty_slot] = UserFee {
            address: *user_address,
            share,
            fee_per_share_checkpoint: self.fee_per_share,
            ..Default::default()
        };

        self.total_share = self.total_share.safe_add(share)?;

        Ok(())
    }

    pub fn validate_and_remove_user_and_get_unclaimed_fee(
        &mut self,
        index: usize,
        user_address: &Pubkey,
    ) -> Result<u64> {
        let user = self
            .users
            .get(index)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(
            user.address.eq(user_address) && user_address.ne(&Pubkey::default()),
            FeeVaultError::InvalidUserAddress
        );

        let user_count = self
            .users
            .iter()
            .filter(|u| u.address.ne(&Pubkey::default()))
            .count();
        // user_count include the user being removed. after removal user count should be at least MIN_USER
        require!(user_count > MIN_USER, FeeVaultError::InvalidNumberOfUsers);

        let unclaimed_fee = user.get_total_pending_fee(self.fee_per_share)?;

        self.total_share = self.total_share.safe_sub(user.share)?;

        // shift users to the left
        for i in index..MAX_USER - 1 {
            self.users[i] = self.users[i + 1];
        }
        self.users[MAX_USER - 1] = UserFee::default();

        Ok(unclaimed_fee)
    }
}

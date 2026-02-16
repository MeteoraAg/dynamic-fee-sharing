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
    pub mutable_flag: u8, // indicate whether the fee vault is mutable by admin or operator
    pub padding_0: [u8; 12],
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
    pub fn get_pending_fee(&self, fee_per_share: u128) -> Result<u64> {
        let delta = fee_per_share.safe_sub(self.fee_per_share_checkpoint)?;
        mul_shr(self.share.into(), delta, PRECISION_SCALE)
            .and_then(|fee| fee.try_into().ok())
            .ok_or(FeeVaultError::MathOverflow.into())
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

    pub fn validate_and_claim_fee(&mut self, index: usize, signer: &Pubkey) -> Result<u64> {
        let user = self
            .users
            .get_mut(index as usize)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(user.address.eq(signer), FeeVaultError::InvalidUserAddress);

        let current_pending_fee = user.get_pending_fee(self.fee_per_share)?;
        let fee_being_claimed = user.pending_fee.safe_add(current_pending_fee)?;

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

    pub fn validate_and_update_share(&mut self, index: usize, share: u32) -> Result<()> {
        require!(
            index < MAX_USER && self.users[index].address != Pubkey::default(),
            FeeVaultError::InvalidUserIndex
        );
        require!(share > 0, FeeVaultError::InvalidFeeVaultParameters);

        // when updating user share, we need to update the pending fee for all users
        // based on the current fee per share to preserve the fee distribution up to that point
        let mut total_share = 0;
        for (i, user) in self.users.iter_mut().enumerate() {
            if user.address == Pubkey::default() {
                continue;
            }

            let current_pending_fee = user.get_pending_fee(self.fee_per_share)?;
            user.pending_fee = user.pending_fee.safe_add(current_pending_fee)?;
            user.fee_per_share_checkpoint = self.fee_per_share;

            if i == index as usize {
                require!(
                    share != user.share,
                    FeeVaultError::InvalidFeeVaultParameters
                );
                user.share = share;
            }
            total_share = total_share.safe_add(user.share)?;
        }
        self.total_share = total_share;

        Ok(())
    }

    pub fn validate_and_remove_user(&mut self, index: usize) -> Result<()> {
        require!(
            index < MAX_USER && self.users[index].address != Pubkey::default(),
            FeeVaultError::InvalidUserIndex
        );

        let mut unclaimed_fee = 0;
        let mut total_share = 0;
        let mut remaining_number_of_users = 0;
        for (i, user) in self.users.iter().enumerate() {
            if user.address == Pubkey::default() {
                continue;
            }

            if i == index as usize {
                let current_pending_fee = user.get_pending_fee(self.fee_per_share)?;
                unclaimed_fee = user.pending_fee.safe_add(current_pending_fee)?;
            } else {
                remaining_number_of_users = remaining_number_of_users.safe_add(1)?;
                total_share = total_share.safe_add(user.share)?;
            }
        }

        require!(
            remaining_number_of_users >= 2,
            FeeVaultError::InvalidNumberOfUsers
        );

        self.total_share = total_share;

        // redistribute removed user's total unclaimed fees
        if unclaimed_fee > 0 {
            let fee_per_share_increase =
                shl_div(unclaimed_fee, total_share.into(), PRECISION_SCALE)
                    .ok_or_else(|| FeeVaultError::MathOverflow)?;
            self.fee_per_share = self.fee_per_share.safe_add(fee_per_share_increase)?;
        }

        // shift users to the left
        for i in index..MAX_USER - 1 {
            self.users[i] = self.users[i + 1];
        }
        self.users[MAX_USER - 1] = UserFee::default();

        Ok(())
    }
}

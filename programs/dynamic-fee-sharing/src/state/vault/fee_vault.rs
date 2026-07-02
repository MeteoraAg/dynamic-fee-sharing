use crate::{
    constants::{MAX_FEE_VAULT_USER, PRECISION_SCALE},
    error::FeeVaultError,
    math::{mul_shr, shl_div, SafeMath},
    params::UserShare,
    state::{VaultHeader, VaultOps},
};
use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;
use std::cell::RefMut;

#[account(zero_copy)]
#[derive(InitSpace, Debug, Default)]
pub struct FeeVault {
    pub fixed: VaultHeader,
    pub users: [UserFee; MAX_FEE_VAULT_USER],
}
const_assert_eq!(FeeVault::INIT_SPACE, 640);

impl std::ops::Deref for FeeVault {
    type Target = VaultHeader;
    fn deref(&self) -> &Self::Target {
        &self.fixed
    }
}

impl std::ops::DerefMut for FeeVault {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fixed
    }
}

#[zero_copy]
#[derive(InitSpace, Debug, Default)]
pub struct UserFee {
    pub address: Pubkey,
    pub share: u32,
    pub padding_0: [u8; 4],
    pub fee_claimed: u64,
    pub pending_fee: u64, // never mutated on FeeVault (no update-share/remove-user ix); kept for on-chain layout compatibility
    pub padding_1: [u8; 8], // padding for future use
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

impl VaultOps for RefMut<'_, FeeVault> {
    fn validate_token_accounts(&self, token_vault: &Pubkey, token_mint: &Pubkey) -> Result<bool> {
        require!(
            self.fixed.token_vault.eq(token_vault) && self.fixed.token_mint.eq(token_mint),
            FeeVaultError::InvalidFeeVault
        );

        Ok(true)
    }

    fn fund_fee(&mut self, _is_token_0: bool, amount: u64) -> Result<u128> {
        let vault = &mut **self;

        vault.fixed.total_funded_fee = vault.fixed.total_funded_fee.safe_add(amount)?;

        let fee_per_share = shl_div(amount, vault.fixed.total_share.into(), PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        vault.fixed.fee_per_share = vault.fixed.fee_per_share.safe_add(fee_per_share)?;

        Ok(vault.fixed.fee_per_share)
    }

    fn validate_and_claim_fee(
        &mut self,
        index: usize,
        _is_token_0: bool,
        signer: &Pubkey,
    ) -> Result<u64> {
        let vault = &mut **self;

        let user = vault
            .users
            .get_mut(index)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(user.address.eq(signer), FeeVaultError::InvalidUserAddress);

        let fee_being_claimed = user.get_total_pending_fee(vault.fixed.fee_per_share)?;

        user.fee_per_share_checkpoint = vault.fixed.fee_per_share;
        user.pending_fee = 0;
        user.fee_claimed = user.fee_claimed.safe_add(fee_being_claimed)?;

        Ok(fee_being_claimed)
    }
}

impl FeeVault {
    pub fn is_share_holder(&self, signer: &Pubkey) -> bool {
        self.users
            .iter()
            .any(|share_holder| share_holder.address.eq(signer))
    }

    pub fn initialize(
        &mut self,
        owner: &Pubkey,
        token_flag: u8,
        token_mint: &Pubkey,
        token_vault: &Pubkey,
        base: &Pubkey,
        vault_bump: u8,
        vault_type: u8,
        users: &[UserShare],
    ) -> Result<()> {
        self.fixed.initialize(
            owner,
            token_flag,
            token_mint,
            token_vault,
            base,
            vault_bump,
            vault_type,
        );

        let mut total_share = 0;
        for i in 0..users.len() {
            self.users[i] =
                UserFee::new(users[i].address, users[i].share, self.fixed.fee_per_share);
            total_share = total_share.safe_add(users[i].share)?;
        }
        self.fixed.total_share = total_share;

        Ok(())
    }
}

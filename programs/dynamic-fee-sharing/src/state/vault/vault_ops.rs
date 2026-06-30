use crate::{
    constants::PRECISION_SCALE,
    error::FeeVaultError,
    math::{mul_shr, shl_div, SafeMath},
    state::{DynamicFeeVault, FeeVault, UserFee, VaultHeader},
    utils::DynamicAccountMut,
};
use anchor_lang::prelude::*;

// Shared logic between FeeVault and DyanmicFeeVault
pub trait VaultOps {
    fn get_header_and_users(&self) -> (&VaultHeader, &[UserFee]);
    fn get_header_and_users_mut(&mut self) -> (&mut VaultHeader, &mut [UserFee]);

    fn validate_token_accounts(&self, token_vault: &Pubkey, token_mint: &Pubkey) -> Result<()> {
        let (header, _) = self.get_header_and_users();

        require!(
            header.token_vault.eq(token_vault) && header.token_mint.eq(token_mint),
            FeeVaultError::InvalidFeeVault
        );

        Ok(())
    }

    fn fund_fee(&mut self, amount: u64) -> Result<()> {
        let (header, _) = self.get_header_and_users_mut();

        header.total_funded_fee = header.total_funded_fee.safe_add(amount)?;

        let fee_per_share = shl_div(amount, header.total_share.into(), PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        header.fee_per_share = header.fee_per_share.safe_add(fee_per_share)?;

        Ok(())
    }

    fn is_share_holder(&self, signer: &Pubkey) -> bool {
        let (_, users) = self.get_header_and_users();

        users
            .iter()
            .any(|share_holder| share_holder.address.eq(signer))
    }

    fn validate_and_claim_fee(&mut self, index: u8, signer: &Pubkey) -> Result<u64> {
        let (header, users) = self.get_header_and_users_mut();

        let user = users
            .get_mut(index as usize)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(user.address.eq(signer), FeeVaultError::InvalidUserAddress);

        let reward_per_share_delta = header
            .fee_per_share
            .safe_sub(user.fee_per_share_checkpoint)?;

        let fee_being_claimed = mul_shr(user.share.into(), reward_per_share_delta, PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?
            .try_into()
            .map_err(|_| FeeVaultError::MathOverflow)?;

        user.fee_per_share_checkpoint = header.fee_per_share;
        user.fee_claimed = user.fee_claimed.safe_add(fee_being_claimed)?;

        Ok(fee_being_claimed)
    }
}

impl VaultOps for FeeVault {
    fn get_header_and_users(&self) -> (&VaultHeader, &[UserFee]) {
        (&self.fixed, &self.users)
    }
    fn get_header_and_users_mut(&mut self) -> (&mut VaultHeader, &mut [UserFee]) {
        (&mut self.fixed, &mut self.users)
    }
}

impl VaultOps for DynamicAccountMut<'_, DynamicFeeVault, UserFee> {
    fn get_header_and_users(&self) -> (&VaultHeader, &[UserFee]) {
        (&self.fixed, &self.dynamic)
    }
    fn get_header_and_users_mut(&mut self) -> (&mut VaultHeader, &mut [UserFee]) {
        (&mut self.fixed, &mut self.dynamic)
    }
}

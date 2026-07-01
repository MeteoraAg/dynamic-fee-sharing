use crate::{
    constants::PRECISION_SCALE,
    error::FeeVaultError,
    math::{shl_div, SafeMath},
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

    // allow new user to be added with 0 share
    fn validate_add_user(&self, user: &Pubkey, max_user: usize) -> Result<()> {
        require!(
            user.ne(&Pubkey::default()),
            FeeVaultError::InvalidUserAddress
        );

        let (_, users) = self.get_header_and_users();
        require!(users.len() < max_user, FeeVaultError::ExceededUser);

        require!(
            !self.is_share_holder(user),
            FeeVaultError::DuplicatedUserAddress
        );

        Ok(())
    }

    fn validate_and_claim_fee(&mut self, index: u8, signer: &Pubkey) -> Result<u64> {
        let (header, users) = self.get_header_and_users_mut();

        let user = users
            .get_mut(index as usize)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(user.address.eq(signer), FeeVaultError::InvalidUserAddress);

        let fee_being_claimed = user.get_total_pending_fee(header.fee_per_share)?;

        user.fee_per_share_checkpoint = header.fee_per_share;
        user.pending_fee = 0;
        user.fee_claimed = user.fee_claimed.safe_add(fee_being_claimed)?;

        Ok(fee_being_claimed)
    }

    /// returns old_share
    fn validate_and_update_share(&mut self, index: u8, signer: &Pubkey, share: u32) -> Result<u32> {
        let (header, users) = self.get_header_and_users_mut();

        let user = users
            .get_mut(index as usize)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;

        require!(
            user.address.eq(signer) && signer.ne(&Pubkey::default()),
            FeeVaultError::InvalidUserAddress
        );

        let old_share = user.share;

        user.pending_fee = user.get_total_pending_fee(header.fee_per_share)?;
        user.fee_per_share_checkpoint = header.fee_per_share;
        user.share = share; // share can be set to 0

        header.total_share = header.total_share.safe_sub(old_share)?.safe_add(share)?;

        require!(
            header.total_share > 0,
            FeeVaultError::InvalidFeeVaultParameters
        );

        Ok(old_share)
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

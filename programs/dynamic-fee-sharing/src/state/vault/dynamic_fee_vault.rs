use crate::constants::{MAX_DYNAMIC_FEE_VAULT_USER, MIN_USER, PRECISION_SCALE};
use crate::error::FeeVaultError;
use crate::math::{mul_shr, shl_div, SafeMath};
use crate::state::{DynamicVaultHeader, VaultOps};
use crate::utils::{d_load_mut_checked, DynamicAccountMut};
use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};
use static_assertions::const_assert_eq;

#[account(zero_copy)]
#[derive(InitSpace, Debug, Default)]
#[repr(C, align(8))]
pub struct DynamicFeeVault {
    pub fixed: DynamicVaultHeader,
}
const_assert_eq!(DynamicFeeVault::INIT_SPACE, 320);
const_assert_eq!((8 + DynamicFeeVault::INIT_SPACE) % 8, 0);
const_assert_eq!(DynamicUserFee::INIT_SPACE % 8, 0);

impl std::ops::Deref for DynamicFeeVault {
    type Target = DynamicVaultHeader;
    fn deref(&self) -> &Self::Target {
        &self.fixed
    }
}

impl std::ops::DerefMut for DynamicFeeVault {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fixed
    }
}

impl DynamicFeeVault {
    pub fn space(num_users: usize) -> usize {
        8 + DynamicFeeVault::INIT_SPACE + num_users * DynamicUserFee::INIT_SPACE
    }
}

#[zero_copy]
#[derive(InitSpace, Debug, Default)]
pub struct DynamicUserFee {
    pub address: Pubkey,
    pub share: u32,
    pub padding_0: [u8; 4],
    pub fee_claimed_token_0: u64,
    pub fee_claimed_token_1: u64,
    pub pending_fee_token_0: u64,
    pub pending_fee_token_1: u64,
    pub padding_1: [u8; 8],
    pub fee_per_share_checkpoint_token_0: u128,
    pub fee_per_share_checkpoint_token_1: u128,
    pub padding_2: [u8; 16],
}

// TODO: doesnt need to be 128
const_assert_eq!(DynamicUserFee::INIT_SPACE, 128);

impl DynamicUserFee {
    pub fn new(
        address: Pubkey,
        share: u32,
        fee_per_share_checkpoint_token_0: u128,
        fee_per_share_checkpoint_token_1: u128,
    ) -> Self {
        Self {
            address,
            share,
            fee_per_share_checkpoint_token_0,
            fee_per_share_checkpoint_token_1,
            ..Default::default()
        }
    }

    pub fn get_total_pending_fee(&self, is_token_0: bool, fee_per_share: u128) -> Result<u64> {
        let (fee_per_share_checkpoint, pending_fee) = if is_token_0 {
            (
                self.fee_per_share_checkpoint_token_0,
                self.pending_fee_token_0,
            )
        } else {
            (
                self.fee_per_share_checkpoint_token_1,
                self.pending_fee_token_1,
            )
        };

        let delta = fee_per_share.safe_sub(fee_per_share_checkpoint)?;
        let current_pending_fee = mul_shr(self.share.into(), delta, PRECISION_SCALE)
            .and_then(|fee| fee.try_into().ok())
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        let total_pending_fee = pending_fee.safe_add(current_pending_fee)?;
        Ok(total_pending_fee)
    }
}

// Indexed by token slot (0 or 1). Slot 1 is unused when token_1_mint == Pubkey::default();
// fee_per_share_token_1 then never grows, so per-user accrual naturally stays zero without
// extra branching.
impl DynamicAccountMut<'_, DynamicFeeVault, DynamicUserFee> {
    fn get_header_and_users(&self) -> (&DynamicVaultHeader, &[DynamicUserFee]) {
        (&self.fixed.fixed, &self.dynamic)
    }

    fn get_header_and_users_mut(&mut self) -> (&mut DynamicVaultHeader, &mut [DynamicUserFee]) {
        (&mut self.fixed.fixed, &mut self.dynamic)
    }

    pub fn is_share_holder(&self, signer: &Pubkey) -> bool {
        let (_, users) = self.get_header_and_users();

        users
            .iter()
            .any(|share_holder| share_holder.address.eq(signer))
    }

    // allow new user to be added with 0 share
    pub fn validate_add_user(&self, user: &Pubkey, max_user: usize) -> Result<()> {
        require!(
            user.ne(&Pubkey::default()),
            FeeVaultError::InvalidUserAddress
        );

        let (_, users) = self.get_header_and_users();
        require!(users.len() < max_user, FeeVaultError::InvalidNumberOfUsers);

        require!(
            !self.is_share_holder(user),
            FeeVaultError::DuplicatedUserAddress
        );

        Ok(())
    }

    /// returns old_share
    pub fn validate_and_update_share(
        &mut self,
        index: usize,
        user_being_updated: &Pubkey,
        share: u32,
    ) -> Result<u32> {
        let (header, users) = self.get_header_and_users_mut();

        let user = users
            .get_mut(index)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;

        require!(
            user.address.eq(user_being_updated) && user_being_updated.ne(&Pubkey::default()),
            FeeVaultError::InvalidUserAddress
        );

        let old_share = user.share;

        user.pending_fee_token_0 =
            user.get_total_pending_fee(true, header.fee_per_share_token_0)?;
        user.fee_per_share_checkpoint_token_0 = header.fee_per_share_token_0;

        user.pending_fee_token_1 =
            user.get_total_pending_fee(false, header.fee_per_share_token_1)?;
        user.fee_per_share_checkpoint_token_1 = header.fee_per_share_token_1;
        user.share = share; // share can be set to 0

        header.total_share = header.total_share.safe_sub(old_share)?.safe_add(share)?;

        require!(header.total_share > 0, FeeVaultError::TotalShareIsZero);

        Ok(old_share)
    }

    /// returns unclaimed_fee per token
    pub fn validate_and_remove_user(
        &mut self,
        index: usize,
        user_being_removed: &Pubkey,
    ) -> Result<(u64, u64)> {
        require!(
            user_being_removed.ne(&Pubkey::default()),
            FeeVaultError::InvalidUserAddress
        );

        let (header, users) = self.get_header_and_users_mut();

        // user count includes the user being removed; must stay at least MIN_USER after removal
        require!(users.len() > MIN_USER, FeeVaultError::InvalidNumberOfUsers);

        let user = users
            .get(index)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(
            user.address.eq(user_being_removed),
            FeeVaultError::InvalidUserAddress
        );

        let unclaimed_fee_0 = user.get_total_pending_fee(true, header.fee_per_share_token_0)?;
        let unclaimed_fee_1 = user.get_total_pending_fee(false, header.fee_per_share_token_1)?;

        header.total_share = header.total_share.safe_sub(user.share)?;
        require!(header.total_share > 0, FeeVaultError::TotalShareIsZero);

        let last_index = users.len().safe_sub(1)?;
        for i in index..last_index {
            users[i] = users[i.safe_add(1)?];
        }

        Ok((unclaimed_fee_0, unclaimed_fee_1))
    }
}

impl VaultOps for DynamicAccountMut<'_, DynamicFeeVault, DynamicUserFee> {
    fn validate_token_accounts(&self, token_vault: &Pubkey, token_mint: &Pubkey) -> Result<bool> {
        let (header, _) = self.get_header_and_users();

        if header.token_0_vault.eq(token_vault) && header.token_0_mint.eq(token_mint) {
            Ok(true)
        } else if header.token_1_vault.eq(token_vault) && header.token_1_mint.eq(token_mint) {
            Ok(false)
        } else {
            Err(FeeVaultError::InvalidFeeVault.into())
        }
    }

    fn fund_fee(&mut self, is_token_0: bool, amount: u64) -> Result<u128> {
        let (header, _) = self.get_header_and_users_mut();

        let fee_per_share = shl_div(amount, header.total_share.into(), PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        if is_token_0 {
            header.total_funded_fee_token_0 = header.total_funded_fee_token_0.safe_add(amount)?;
            header.fee_per_share_token_0 = header.fee_per_share_token_0.safe_add(fee_per_share)?;
            Ok(header.fee_per_share_token_0)
        } else {
            header.total_funded_fee_token_1 = header.total_funded_fee_token_1.safe_add(amount)?;
            header.fee_per_share_token_1 = header.fee_per_share_token_1.safe_add(fee_per_share)?;
            Ok(header.fee_per_share_token_1)
        }
    }

    fn validate_and_claim_fee(
        &mut self,
        index: usize,
        is_token_0: bool,
        signer: &Pubkey,
    ) -> Result<u64> {
        let (header, users) = self.get_header_and_users_mut();

        let user = users
            .get_mut(index)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(user.address.eq(signer), FeeVaultError::InvalidUserAddress);

        if is_token_0 {
            let fee_being_claimed =
                user.get_total_pending_fee(true, header.fee_per_share_token_0)?;
            user.fee_per_share_checkpoint_token_0 = header.fee_per_share_token_0;
            user.pending_fee_token_0 = 0;
            user.fee_claimed_token_0 = user.fee_claimed_token_0.safe_add(fee_being_claimed)?;
            Ok(fee_being_claimed)
        } else {
            let fee_being_claimed =
                user.get_total_pending_fee(false, header.fee_per_share_token_1)?;
            user.fee_per_share_checkpoint_token_1 = header.fee_per_share_token_1;
            user.pending_fee_token_1 = 0;
            user.fee_claimed_token_1 = user.fee_claimed_token_1.safe_add(fee_being_claimed)?;
            Ok(fee_being_claimed)
        }
    }
}

fn grow_user_tail<'info>(
    fee_vault_info: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    system_program: Pubkey,
) -> Result<()> {
    let new_len = fee_vault_info
        .data_len()
        .safe_add(DynamicUserFee::INIT_SPACE)?;

    let rent = Rent::get()?;
    let lamports_diff = rent
        .minimum_balance(new_len)
        .saturating_sub(fee_vault_info.lamports());

    if lamports_diff > 0 {
        system_program::transfer(
            CpiContext::new(
                system_program,
                Transfer {
                    from: payer.clone(),
                    to: fee_vault_info.clone(),
                },
            ),
            lamports_diff,
        )?;
    }

    fee_vault_info.resize(new_len)?;

    Ok(())
}

fn shrink_user_tail<'info>(
    fee_vault_info: &AccountInfo<'info>,
    rent_receiver: &AccountInfo<'info>,
) -> Result<()> {
    let new_len = fee_vault_info
        .data_len()
        .safe_sub(DynamicUserFee::INIT_SPACE)?;

    fee_vault_info.resize(new_len)?;

    let rent = Rent::get()?;
    let minimum_balance = rent.minimum_balance(new_len);
    let lamports_diff = fee_vault_info.lamports().safe_sub(minimum_balance)?;

    if lamports_diff > 0 {
        fee_vault_info.sub_lamports(lamports_diff)?;
        rent_receiver.add_lamports(lamports_diff)?;
    }

    Ok(())
}

pub fn add_user_and_grow<'info>(
    fee_vault_loader: &AccountLoader<'info, DynamicFeeVault>,
    payer: &AccountInfo<'info>,
    system_program: Pubkey,
    user: &Pubkey,
    share: u32,
) -> Result<()> {
    let fee_vault_info = fee_vault_loader.to_account_info();

    let vault = d_load_mut_checked::<DynamicFeeVault, DynamicUserFee>(&fee_vault_info)?;
    let fee_per_share_token_0 = vault.fixed.fee_per_share_token_0;
    let fee_per_share_token_1 = vault.fixed.fee_per_share_token_1;
    vault.validate_add_user(user, MAX_DYNAMIC_FEE_VAULT_USER)?;
    drop(vault);

    grow_user_tail(&fee_vault_info, payer, system_program)?;

    let mut vault = d_load_mut_checked::<DynamicFeeVault, DynamicUserFee>(&fee_vault_info)?;
    let last = vault
        .dynamic
        .last_mut()
        .ok_or_else(|| error!(FeeVaultError::InvalidNumberOfUsers))?;
    *last = DynamicUserFee::new(*user, share, fee_per_share_token_0, fee_per_share_token_1);
    vault.fixed.total_share = vault.fixed.total_share.safe_add(share)?;

    Ok(())
}

pub fn remove_user_and_shrink<'info>(
    fee_vault_loader: &AccountLoader<'info, DynamicFeeVault>,
    rent_receiver: &AccountInfo<'info>,
    index: usize,
    user: &Pubkey,
) -> Result<(u64, u64)> {
    let fee_vault_info = fee_vault_loader.to_account_info();

    let mut vault = d_load_mut_checked::<DynamicFeeVault, DynamicUserFee>(&fee_vault_info)?;
    let unclaimed_fee = vault.validate_and_remove_user(index, user)?;
    drop(vault);

    shrink_user_tail(&fee_vault_info, rent_receiver)?;

    Ok(unclaimed_fee)
}

use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};

use crate::constants::{MAX_STATIC_USER, MAX_USER, MIN_USER};
use crate::error::FeeVaultError;
use crate::math::SafeMath;
use crate::state::{FeeVault, UserFee};

/// A fee vault struct loaded with dynamic sized data type
#[derive(Debug)]
pub struct DynamicFeeVault<'a> {
    pub fee_vault: RefMut<'a, FeeVault>,
    dynamic_user_data: RefMut<'a, [UserFee]>,
}

pub trait DynamicFeeVaultLoader<'info> {
    fn load_content_mut(&self) -> Result<DynamicFeeVault>;
}

impl<'info> DynamicFeeVaultLoader<'info> for AccountLoader<'info, FeeVault> {
    fn load_content_mut(&self) -> Result<DynamicFeeVault> {
        fee_vault_account_split(self)
    }
}

fn fee_vault_account_split<'a>(
    fee_vault_account_loader: &'a AccountLoader<FeeVault>,
) -> Result<DynamicFeeVault<'a>> {
    let data = fee_vault_account_loader.as_ref().try_borrow_mut_data()?;

    let (fee_vault, dynamic_user_data) = RefMut::map_split(data, |data| {
        let (fee_vault_bytes, dynamic_user_data_bytes) =
            data.split_at_mut(8 + FeeVault::INIT_SPACE);
        let fee_vault = bytemuck::from_bytes_mut::<FeeVault>(&mut fee_vault_bytes[8..]);
        let dynamic_user_data = bytemuck::cast_slice_mut::<u8, UserFee>(dynamic_user_data_bytes);
        (fee_vault, dynamic_user_data)
    });
    Ok(DynamicFeeVault {
        fee_vault,
        dynamic_user_data,
    })
}

impl DynamicFeeVault<'_> {
    fn get_user(&self, index: usize) -> Result<&UserFee> {
        if index < MAX_STATIC_USER {
            self.fee_vault
                .users
                .get(index)
                .ok_or_else(|| error!(FeeVaultError::InvalidUserIndex))
        } else {
            let dynamic_index = index.safe_sub(MAX_STATIC_USER)?;
            self.dynamic_user_data
                .get(dynamic_index)
                .ok_or_else(|| error!(FeeVaultError::InvalidUserIndex))
        }
    }

    fn get_user_mut(&mut self, index: usize) -> Result<&mut UserFee> {
        if index < MAX_STATIC_USER {
            self.fee_vault
                .users
                .get_mut(index)
                .ok_or_else(|| error!(FeeVaultError::InvalidUserIndex))
        } else {
            let dynamic_index = index.safe_sub(MAX_STATIC_USER)?;
            self.dynamic_user_data
                .get_mut(dynamic_index)
                .ok_or_else(|| error!(FeeVaultError::InvalidUserIndex))
        }
    }

    pub fn is_share_holder(&self, user: &Pubkey) -> bool {
        user.ne(&Pubkey::default())
            && (self.fee_vault.users.iter().any(|u| u.address.eq(user))
                || self.dynamic_user_data.iter().any(|u| u.address.eq(user)))
    }

    pub fn get_user_count(&self) -> usize {
        self.fee_vault
            .users
            .iter()
            .chain(self.dynamic_user_data.iter())
            .filter(|u| u.address.ne(&Pubkey::default()))
            .count()
    }

    // Find the first empty slot in the fixed-size users
    pub fn find_first_empty_slot_in_fixed_users(&self) -> Option<usize> {
        self.fee_vault
            .users
            .iter()
            .position(|u| u.address.eq(&Pubkey::default()))
    }

    pub fn claim_fee(&mut self, index: usize, signer: &Pubkey) -> Result<u64> {
        let fee_per_share = self.fee_vault.fee_per_share;
        let user = self.get_user_mut(index)?;

        require!(user.address.eq(signer), FeeVaultError::InvalidUserAddress);

        let fee_being_claimed = user.get_total_pending_fee(fee_per_share)?;

        user.pending_fee = 0;
        user.fee_per_share_checkpoint = fee_per_share;
        user.fee_claimed = user.fee_claimed.safe_add(fee_being_claimed)?;

        Ok(fee_being_claimed)
    }

    pub fn update_share(&mut self, index: usize, user_address: &Pubkey, share: u32) -> Result<()> {
        let fee_per_share = self.fee_vault.fee_per_share;
        let user = self.get_user_mut(index)?;

        require!(
            user.address.eq(user_address) && user_address.ne(&Pubkey::default()),
            FeeVaultError::InvalidUserAddress
        );

        // share can be set to 0
        let old_share = user.share;

        user.pending_fee = user.get_total_pending_fee(fee_per_share)?;
        user.fee_per_share_checkpoint = fee_per_share;
        user.share = share;

        self.fee_vault.total_share = self
            .fee_vault
            .total_share
            .safe_sub(old_share)?
            .safe_add(share)?;

        require!(
            self.fee_vault.total_share > 0,
            FeeVaultError::InvalidFeeVaultParameters
        );

        Ok(())
    }

    /// removes the user at index and shift-left the user in the arrays that are after the removed slot.
    /// returns whether the dynamic array should shrink after the removal.
    fn remove_user_slot(&mut self, index: usize) -> Result<bool> {
        let dynamic_removal_slot_index = if index < MAX_STATIC_USER {
            // shift fixed users left
            let last_fixed_index = MAX_STATIC_USER.safe_sub(1)?;
            for i in index..last_fixed_index {
                self.fee_vault.users[i] = self.fee_vault.users[i.safe_add(1)?];
            }

            if self.dynamic_user_data.is_empty() {
                self.fee_vault.users[last_fixed_index] = UserFee::default();
                return Ok(false); // return early
            }

            // shift first dynamic user into last fixed slot
            self.fee_vault.users[last_fixed_index] = self.dynamic_user_data[0];
            0
        } else {
            index.safe_sub(MAX_STATIC_USER)?
        };

        // shift dynamic users left
        let dynamic_len = self.dynamic_user_data.len();
        for i in dynamic_removal_slot_index..dynamic_len.safe_sub(1)? {
            self.dynamic_user_data[i] = self.dynamic_user_data[i.safe_add(1)?];
        }
        self.dynamic_user_data[dynamic_len.safe_sub(1)?] = UserFee::default();
        Ok(true)
    }

    pub fn remove_user(&mut self, index: usize, user_address: &Pubkey) -> Result<(u64, bool)> {
        require!(
            user_address.ne(&Pubkey::default()),
            FeeVaultError::InvalidUserAddress
        );

        // user_count includes the user being removed; after removal count should be at least MIN_USER
        require!(
            self.get_user_count() > MIN_USER,
            FeeVaultError::InvalidNumberOfUsers
        );

        let user = self.get_user(index)?;

        require!(
            user.address.eq(user_address),
            FeeVaultError::InvalidUserAddress
        );

        let unclaimed_fee = user.get_total_pending_fee(self.fee_vault.fee_per_share)?;
        self.fee_vault.total_share = self.fee_vault.total_share.safe_sub(user.share)?;
        let should_shrink = self.remove_user_slot(index)?;

        Ok((unclaimed_fee, should_shrink))
    }

    pub fn validate_add_user(&self, user: &Pubkey) -> Result<()> {
        require!(
            user.ne(&Pubkey::default()) && !self.is_share_holder(user),
            FeeVaultError::InvalidUserAddress
        );

        // user_count does not include the user being added; after addition count should be at most MAX_USER
        require!(
            self.get_user_count() < MAX_USER,
            FeeVaultError::InvalidNumberOfUsers
        );

        Ok(())
    }

    pub fn add_user(&mut self, slot: Option<usize>, user: &Pubkey, share: u32) -> Result<()> {
        let new_user = UserFee::new(*user, share, self.fee_vault.fee_per_share);

        if let Some(index) = slot {
            self.fee_vault.users[index] = new_user;
        } else {
            let last = self
                .dynamic_user_data
                .last_mut()
                .ok_or_else(|| error!(FeeVaultError::InvalidNumberOfUsers))?;
            *last = new_user;
        }

        self.fee_vault.total_share = self.fee_vault.total_share.safe_add(share)?;

        Ok(())
    }
}

pub fn grow_dynamic_user<'info>(
    fee_vault_info: &AccountInfo<'info>,
    signer: &Signer<'info>,
    system_program: &Program<'info, System>,
) -> Result<()> {
    let new_len = fee_vault_info.data_len().safe_add(UserFee::INIT_SPACE)?;
    let rent = Rent::get()?;
    let lamports_diff = rent
        .minimum_balance(new_len)
        .saturating_sub(fee_vault_info.lamports());

    system_program::transfer(
        CpiContext::new(
            system_program.to_account_info(),
            Transfer {
                from: signer.to_account_info(),
                to: fee_vault_info.clone(),
            },
        ),
        lamports_diff,
    )?;

    // we won't read this new space before writing to it
    fee_vault_info.realloc(new_len, false)?;

    Ok(())
}

pub fn shrink_dynamic_user<'info>(
    fee_vault_info: &AccountInfo<'info>,
    rent_receiver: &AccountInfo<'info>,
) -> Result<()> {
    let new_len = fee_vault_info.data_len().safe_sub(UserFee::INIT_SPACE)?;

    fee_vault_info.realloc(new_len, false)?;

    let rent = Rent::get()?;
    let minimum_balance = rent.minimum_balance(new_len);
    let lamports_diff = fee_vault_info.lamports().safe_sub(minimum_balance)?;

    if lamports_diff > 0 {
        fee_vault_info.sub_lamports(lamports_diff)?;
        rent_receiver.add_lamports(lamports_diff)?;
    }

    Ok(())
}

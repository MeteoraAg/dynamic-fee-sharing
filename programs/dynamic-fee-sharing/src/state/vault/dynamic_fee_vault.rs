use crate::constants::MAX_DYNAMIC_FEE_VAULT_USER;
use crate::error::FeeVaultError;
use crate::math::SafeMath;
use crate::state::{UserFee, VaultHeader, VaultOps};
use crate::utils::d_load_mut_checked;
use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};
use static_assertions::const_assert_eq;

#[account(zero_copy)]
#[derive(InitSpace, Debug, Default)]
#[repr(C, align(8))]
pub struct DynamicFeeVault {
    pub fixed: VaultHeader,
}
const_assert_eq!(DynamicFeeVault::INIT_SPACE, 240);
const_assert_eq!((8 + DynamicFeeVault::INIT_SPACE) % 8, 0);
const_assert_eq!(UserFee::INIT_SPACE % 8, 0);

impl std::ops::Deref for DynamicFeeVault {
    type Target = VaultHeader;
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
        8 + DynamicFeeVault::INIT_SPACE + num_users * UserFee::INIT_SPACE
    }

    pub fn initialize_header(
        &mut self,
        owner: &Pubkey,
        token_flag: u8,
        token_mint: &Pubkey,
        token_vault: &Pubkey,
        base: &Pubkey,
        vault_bump: u8,
        vault_type: u8,
    ) {
        self.fixed.initialize(
            owner,
            token_flag,
            token_mint,
            token_vault,
            base,
            vault_bump,
            vault_type,
        );
    }
}

fn grow_user_tail<'info>(
    fee_vault_info: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    system_program: Pubkey,
) -> Result<()> {
    let new_len = fee_vault_info.data_len().safe_add(UserFee::INIT_SPACE)?;

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

pub fn add_user_and_grow<'info>(
    fee_vault_loader: &AccountLoader<'info, DynamicFeeVault>,
    payer: &AccountInfo<'info>,
    system_program: Pubkey,
    user: &Pubkey,
    share: u32,
) -> Result<()> {
    let fee_vault_info = fee_vault_loader.to_account_info();

    let vault = d_load_mut_checked::<DynamicFeeVault, UserFee>(&fee_vault_info)?;
    let fee_per_share = vault.fixed.fee_per_share;
    vault.validate_add_user(user, MAX_DYNAMIC_FEE_VAULT_USER)?;
    drop(vault);

    grow_user_tail(&fee_vault_info, payer, system_program)?;

    let mut vault = d_load_mut_checked::<DynamicFeeVault, UserFee>(&fee_vault_info)?;
    let last = vault
        .dynamic
        .last_mut()
        .ok_or_else(|| error!(FeeVaultError::ExceededUser))?;
    *last = UserFee::new(*user, share, fee_per_share);
    vault.fixed.total_share = vault.fixed.total_share.safe_add(share)?;

    Ok(())
}

use crate::error::FeeVaultError;
use crate::event::EvtUpdateUserShare;
use crate::state::{DynamicFeeVault, UserFee, VaultOps};
use crate::utils::d_load_mut_checked;
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct UpdateUserShareCtx<'info> {
    #[account(mut, has_one = owner @ FeeVaultError::InvalidSigner)]
    pub fee_vault: AccountLoader<'info, DynamicFeeVault>,

    /// CHECK: the user whose share is being updated
    pub user: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

pub fn handle_update_user_share(
    ctx: Context<UpdateUserShareCtx>,
    index: u8,
    share: u32,
) -> Result<()> {
    let user = ctx.accounts.user.key();

    let fee_vault_info = ctx.accounts.fee_vault.to_account_info();
    let mut vault = d_load_mut_checked::<DynamicFeeVault, UserFee>(&fee_vault_info)?;
    let old_share = vault.validate_and_update_share(index, &user, share)?;
    drop(vault);

    emit_cpi!(EvtUpdateUserShare {
        fee_vault: ctx.accounts.fee_vault.key(),
        user,
        old_share,
        new_share: share,
    });

    Ok(())
}

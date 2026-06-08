use crate::event::EvtAddUser;
use crate::state::{grow_dynamic_user, DynamicFeeVaultLoader, FeeVault};
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct AddUserCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: the user being added
    pub user: UncheckedAccount<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_add_user(ctx: Context<AddUserCtx>, share: u32) -> Result<()> {
    let fee_vault_info = ctx.accounts.fee_vault.as_ref().to_account_info();

    let user = ctx.accounts.user.key();

    let empty_slot = {
        let vault = ctx.accounts.fee_vault.load_content_mut()?;
        vault.validate_add_user(&user)?;
        vault.find_first_empty_slot_in_fixed_users()
    };

    if empty_slot.is_none() {
        grow_dynamic_user(
            &fee_vault_info,
            &ctx.accounts.signer,
            ctx.accounts.system_program.key(),
        )?;
    }

    let mut vault = ctx.accounts.fee_vault.load_content_mut()?;
    vault.add_user(empty_slot, &user, share)?;

    emit_cpi!(EvtAddUser {
        fee_vault: ctx.accounts.fee_vault.key(),
        user,
        share,
    });

    Ok(())
}

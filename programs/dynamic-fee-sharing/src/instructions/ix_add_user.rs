use crate::error::FeeVaultError;
use crate::event::EvtAddUser;
use crate::state::{add_user_and_grow, DynamicFeeVault};
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct AddUserCtx<'info> {
    #[account(mut, has_one = owner @ FeeVaultError::InvalidSigner)]
    pub fee_vault: AccountLoader<'info, DynamicFeeVault>,

    /// CHECK: user being added
    pub user: UncheckedAccount<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_add_user(ctx: Context<AddUserCtx>, share: u32) -> Result<()> {
    let user = ctx.accounts.user.key();

    add_user_and_grow(
        &ctx.accounts.fee_vault,
        &ctx.accounts.owner.to_account_info(),
        ctx.accounts.system_program.key(),
        &user,
        share,
    )?;

    emit_cpi!(EvtAddUser {
        fee_vault: ctx.accounts.fee_vault.key(),
        user,
        share,
    });

    Ok(())
}

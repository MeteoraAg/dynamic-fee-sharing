use crate::event::EvtAddUser;
use crate::state::{add_user_and_grow_if_needed, FeeVault};
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
    let user = ctx.accounts.user.key();

    add_user_and_grow_if_needed(
        &ctx.accounts.fee_vault,
        &ctx.accounts.signer,
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

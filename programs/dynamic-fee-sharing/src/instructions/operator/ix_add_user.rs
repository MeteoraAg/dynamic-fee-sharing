use crate::event::EvtAddUser;
use crate::state::FeeVault;
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct AddUserCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: the user being added
    pub user: UncheckedAccount<'info>,

    pub signer: Signer<'info>,
}

pub fn handle_add_user(ctx: Context<AddUserCtx>, share: u32) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
    let user = ctx.accounts.user.key();
    fee_vault.validate_and_add_user(&user, share)?;

    emit_cpi!(EvtAddUser {
        fee_vault: ctx.accounts.fee_vault.key(),
        user,
        share,
    });

    Ok(())
}

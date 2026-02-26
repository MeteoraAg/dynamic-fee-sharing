use crate::event::EvtUpdateUserShare;
use crate::state::FeeVault;
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct UpdateUserShareCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: the user whose share is being updated
    pub user: UncheckedAccount<'info>,

    pub signer: Signer<'info>,
}

pub fn handle_update_user_share(ctx: Context<UpdateUserShareCtx>, share: u32) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
    let user = ctx.accounts.user.key();
    fee_vault.validate_and_update_share(&user, share)?;

    emit_cpi!(EvtUpdateUserShare {
        fee_vault: ctx.accounts.fee_vault.key(),
        user,
        share,
    });

    Ok(())
}

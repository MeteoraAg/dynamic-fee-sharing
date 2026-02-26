use crate::event::EvtRemoveUser;
use crate::state::FeeVault;
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct RemoveUserCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    pub signer: Signer<'info>,
}

pub fn handle_remove_user(ctx: Context<RemoveUserCtx>, user: Pubkey) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;

    fee_vault.validate_and_remove_user(&user)?;

    emit_cpi!(EvtRemoveUser {
        fee_vault: ctx.accounts.fee_vault.key(),
        user,
    });

    Ok(())
}

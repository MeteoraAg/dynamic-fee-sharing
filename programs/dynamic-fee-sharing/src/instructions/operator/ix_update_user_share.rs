use crate::event::EvtUpdateUserShare;
use crate::state::{FeeVault, Operator};
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct UpdateUserShareCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    pub operator: AccountLoader<'info, Operator>,

    pub signer: Signer<'info>,
}

pub fn handle_update_user_share(
    ctx: Context<UpdateUserShareCtx>,
    index: u8,
    share: u32,
) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;

    fee_vault.validate_and_update_share(index, share)?;

    emit_cpi!(EvtUpdateUserShare {
        fee_vault: ctx.accounts.fee_vault.key(),
        index,
        share,
    });

    Ok(())
}

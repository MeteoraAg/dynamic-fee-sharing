use crate::event::EvtRemoveUser;
use crate::state::FeeVault;
use crate::utils::access_control::verify_is_mutable_and_admin;
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct RemoveUserCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    pub signer: Signer<'info>,
}

pub fn handle_remove_user(ctx: Context<RemoveUserCtx>, index: u8) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;

    verify_is_mutable_and_admin(&fee_vault, &ctx.accounts.signer)?;

    fee_vault.validate_and_remove_user(index as usize)?;

    emit_cpi!(EvtRemoveUser {
        fee_vault: ctx.accounts.fee_vault.key(),
        index,
    });

    Ok(())
}

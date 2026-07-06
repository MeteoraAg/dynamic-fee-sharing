use crate::event::EvtCloseWhitelistedAction;
use crate::state::WhitelistedAction;
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct CloseWhitelistedActionCtx<'info> {
    #[account(mut, close = rent_receiver)]
    pub whitelisted_action: Account<'info, WhitelistedAction>,

    pub admin: Signer<'info>,

    /// CHECK: receives rent of the closed claim action account
    #[account(mut)]
    pub rent_receiver: UncheckedAccount<'info>,
}

pub fn handle_close_whitelisted_action(ctx: Context<CloseWhitelistedActionCtx>) -> Result<()> {
    emit_cpi!(EvtCloseWhitelistedAction {
        whitelisted_action: ctx.accounts.whitelisted_action.key(),
        source_program: ctx.accounts.whitelisted_action.source_program,
        discriminator: ctx.accounts.whitelisted_action.discriminator,
    });

    Ok(())
}

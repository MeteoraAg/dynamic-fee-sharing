use crate::constants::seeds::WHITELISTED_ACTION_PREFIX;
use crate::event::EvtCreateWhitelistedAction;
use crate::params::CreateWhitelistedActionParameters;
use crate::state::WhitelistedAction;
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
#[instruction(params: CreateWhitelistedActionParameters)]
pub struct CreateWhitelistedActionCtx<'info> {
    #[account(
        init,
        seeds = [
            WHITELISTED_ACTION_PREFIX.as_ref(),
            params.source_program.as_ref(),
            params.discriminator.as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + WhitelistedAction::INIT_SPACE
    )]
    pub whitelisted_action: Account<'info, WhitelistedAction>,

    pub admin: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_create_whitelisted_action(
    ctx: Context<CreateWhitelistedActionCtx>,
    params: &CreateWhitelistedActionParameters,
) -> Result<()> {
    params.validate()?;

    let whitelisted_action = &mut ctx.accounts.whitelisted_action;
    whitelisted_action.initialize(params);

    emit_cpi!(EvtCreateWhitelistedAction {
        whitelisted_action: ctx.accounts.whitelisted_action.key(),
        source_program: params.source_program,
        discriminator: params.discriminator,
        token_0_vault_index: params.token_0_vault_index,
        token_1_vault_index: params.token_1_vault_index,
    });

    Ok(())
}

use crate::{error::FeeVaultError, event::EvtUpdateOperator, state::FeeVault};
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct UpdateOperatorCtx<'info> {
    #[account(mut, has_one = owner)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: can be any address
    pub operator: UncheckedAccount<'info>,

    pub owner: Signer<'info>,
}

pub fn handle_update_operator(ctx: Context<UpdateOperatorCtx>) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;

    require!(
        ctx.accounts.operator.key() != fee_vault.operator
            && ctx.accounts.operator.key() != fee_vault.owner,
        FeeVaultError::InvalidOperatorAddress
    );

    fee_vault.operator = ctx.accounts.operator.key();

    emit_cpi!(EvtUpdateOperator {
        fee_vault: ctx.accounts.fee_vault.key(),
        operator: ctx.accounts.operator.key(),
    });

    Ok(())
}

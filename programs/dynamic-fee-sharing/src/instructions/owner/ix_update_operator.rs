use crate::state::FeeVault;
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct UpdateOperatorAccountCtx<'info> {
    #[account(mut, has_one = owner)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: can be any address
    pub operator: UncheckedAccount<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,
}

pub fn handle_update_operator(ctx: Context<UpdateOperatorAccountCtx>) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;

    fee_vault.operator = ctx.accounts.operator.key();

    Ok(())
}

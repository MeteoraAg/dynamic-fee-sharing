use crate::state::{FeeVault, Operator};
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct CloseOperatorAccountCtx<'info> {
    #[account(mut, has_one = owner)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    #[account(
        mut,
        close = rent_receiver
    )]
    pub operator: AccountLoader<'info, Operator>,

    pub owner: Signer<'info>,

    /// CHECK: Account to receive closed account rental SOL
    #[account(mut)]
    pub rent_receiver: UncheckedAccount<'info>,
}

pub fn handle_close_operator_account(ctx: Context<CloseOperatorAccountCtx>) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
    fee_vault.operator_address = Pubkey::default();

    Ok(())
}

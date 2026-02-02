use crate::{
    constants::{seeds::OPERATOR_PREFIX, MAX_OPERATION},
    error::FeeVaultError,
    state::{FeeVault, Operator},
};
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct CreateOperatorAccountCtx<'info> {
    #[account(mut, has_one = owner)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    #[account(
        init,
        payer = owner,
        seeds = [
            OPERATOR_PREFIX.as_ref(),
            whitelisted_address.key().as_ref(),
        ],
        bump,
        space = 8 + Operator::INIT_SPACE
    )]
    pub operator: AccountLoader<'info, Operator>,

    /// CHECK: can be any address
    pub whitelisted_address: UncheckedAccount<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_create_operator_account(
    ctx: Context<CreateOperatorAccountCtx>,
    permission: u128,
) -> Result<()> {
    // validate permission, only support 1 operations for now
    require!(
        permission > 0 && permission < 1 << MAX_OPERATION,
        FeeVaultError::InvalidPermission
    );

    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;

    require!(
        fee_vault.operator_address == Pubkey::default(),
        FeeVaultError::OperatorAlreadyExists
    );

    let mut operator = ctx.accounts.operator.load_init()?;
    operator.initialize(ctx.accounts.whitelisted_address.key(), permission);

    fee_vault.operator_address = ctx.accounts.operator.key();

    Ok(())
}

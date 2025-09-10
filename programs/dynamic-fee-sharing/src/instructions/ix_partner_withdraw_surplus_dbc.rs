use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

use crate::{error::FeeVaultError, math::SafeMath, state::FeeVault};

#[event_cpi]
#[derive(Accounts)]
pub struct WithdrawDbcPartnerSurplusCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    pub fee_claimer: Signer<'info>,

    /// CHECK: The pool config
    pub config: UncheckedAccount<'info>,

    /// CHECK: The virtual pool
    pub pool: UncheckedAccount<'info>,

    /// The treasury token b account
    #[account(mut)]
    pub token_quote_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: The vault token account for quote token
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,

    /// CHECK: The mint of token base
    pub quote_mint: UncheckedAccount<'info>,

    /// CHECK: Token base program
    pub token_base_program: UncheckedAccount<'info>,

    /// CHECK: Token quote program
    pub token_quote_program: UncheckedAccount<'info>,

    /// CHECK: dbc pool authority
    pub dbc_pool_authority: UncheckedAccount<'info>,

    /// CHECK: dbc program
    #[account(address = dynamic_bonding_curve::ID)]
    pub dbc_program: UncheckedAccount<'info>,
    /// CHECK: dammv2 authority
    pub dbc_event_authority: UncheckedAccount<'info>,
}

pub fn handle_withdraw_dbc_partner_surplus(
    ctx: Context<WithdrawDbcPartnerSurplusCtx>,
) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;

    require!(
        fee_vault
            .token_vault
            .eq(&ctx.accounts.token_quote_account.key())
            && fee_vault.token_mint.eq(&ctx.accounts.quote_mint.key()),
        FeeVaultError::InvalidFeeVault
    );

    let before_token_vault_balance = ctx.accounts.token_quote_account.amount;

    dynamic_bonding_curve::cpi::partner_withdraw_surplus(CpiContext::new(
        ctx.accounts.dbc_program.to_account_info(),
        dynamic_bonding_curve::cpi::accounts::PartnerWithdrawSurplus {
            pool_authority: ctx.accounts.dbc_pool_authority.to_account_info(),
            config: ctx.accounts.config.to_account_info(),
            virtual_pool: ctx.accounts.pool.to_account_info(),
            token_quote_account: ctx.accounts.token_quote_account.to_account_info(),
            quote_vault: ctx.accounts.quote_vault.to_account_info(),
            quote_mint: ctx.accounts.quote_mint.to_account_info(),
            fee_claimer: ctx.accounts.fee_claimer.to_account_info(),
            token_quote_program: ctx.accounts.token_quote_program.to_account_info(),
            event_authority: ctx.accounts.dbc_event_authority.to_account_info(),
            program: ctx.accounts.dbc_program.to_account_info(),
        },
    ))?;
    ctx.accounts.token_quote_account.reload()?;

    let after_token_vault_balance = ctx.accounts.token_quote_account.amount;

    let claimed_amount = after_token_vault_balance.safe_sub(before_token_vault_balance)?;

    fee_vault.fund_fee(claimed_amount)?;

    // TODO emit event

    Ok(())
}

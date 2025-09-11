use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use dynamic_bonding_curve::accounts::PoolConfig;

use crate::{error::FeeVaultError, event::EvtClaimDbcTradingFee, math::SafeMath, state::FeeVault};

#[event_cpi]
#[derive(Accounts)]
pub struct ClaimDbcTradingFeeCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    pub fee_claimer: Signer<'info>,

    #[account(has_one=quote_mint, has_one=fee_claimer)]
    pub config: AccountLoader<'info, PoolConfig>,

    /// CHECK: bdc virtual pool
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,

    /// The treasury token a account
    /// This account use to satisfy accounts context. The pool only has fee in token b account
    #[account(mut)]
    pub token_a_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The treasury token b account
    #[account(mut)]
    pub token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: The vault token account for base token
    #[account(mut)]
    pub base_vault: UncheckedAccount<'info>,

    /// CHECK: The vault token account for quote token
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,

    /// CHECK: The mint of token base
    pub base_mint: UncheckedAccount<'info>,

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

pub fn handle_claim_dbc_trading_fee(ctx: Context<ClaimDbcTradingFeeCtx>) -> Result<()> {
    let config = ctx.accounts.config.load()?;
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
    // support collect fee mode is 0 (only quote token)
    require!(config.collect_fee_mode == 0, FeeVaultError::InvalidDbcPool);

    require!(
        fee_vault
            .token_vault
            .eq(&ctx.accounts.token_b_account.key())
            && fee_vault.token_mint.eq(&ctx.accounts.quote_mint.key()),
        FeeVaultError::InvalidFeeVault
    );

    let before_token_vault_balance = ctx.accounts.token_b_account.amount;

    dynamic_bonding_curve::cpi::claim_trading_fee(
        CpiContext::new(
            ctx.accounts.dbc_program.to_account_info(),
            dynamic_bonding_curve::cpi::accounts::ClaimTradingFee {
                pool_authority: ctx.accounts.dbc_pool_authority.to_account_info(),
                config: ctx.accounts.config.to_account_info(),
                pool: ctx.accounts.pool.to_account_info(),
                token_a_account: ctx.accounts.token_a_account.to_account_info(),
                token_b_account: ctx.accounts.token_b_account.to_account_info(),
                base_vault: ctx.accounts.base_vault.to_account_info(),
                quote_vault: ctx.accounts.quote_vault.to_account_info(),
                base_mint: ctx.accounts.base_mint.to_account_info(),
                quote_mint: ctx.accounts.quote_mint.to_account_info(),
                fee_claimer: ctx.accounts.fee_claimer.to_account_info(),
                token_base_program: ctx.accounts.token_base_program.to_account_info(),
                token_quote_program: ctx.accounts.token_quote_program.to_account_info(),
                event_authority: ctx.accounts.dbc_event_authority.to_account_info(),
                program: ctx.accounts.dbc_program.to_account_info(),
            },
        ),
        0,        // max_amount_a, fee only token b
        u64::MAX, // max_amount_b,
    )?;
    ctx.accounts.token_b_account.reload()?;

    let after_token_vault_balance = ctx.accounts.token_b_account.amount;

    let claimed_amount = after_token_vault_balance.safe_sub(before_token_vault_balance)?;

    fee_vault.fund_fee(claimed_amount)?;

    emit_cpi!(EvtClaimDbcTradingFee {
        fee_vault: ctx.accounts.fee_vault.key(),
        pool: ctx.accounts.pool.key(),
        fee_per_share: fee_vault.fee_per_share,
        claimed_amount,
    });

    Ok(())
}

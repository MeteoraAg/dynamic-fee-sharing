use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use dynamic_bonding_curve::accounts::PoolConfig;

use crate::{
    error::FeeVaultError,
    handle_funding_fee,
    state::{FeeVault, FundingType},
};

#[derive(Accounts)]
pub struct FundingByClaimDbcCreatorTradingFeeCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    #[account(has_one = quote_mint)]
    pub config: AccountLoader<'info, PoolConfig>,

    /// CHECK: The virtual pool
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,

    /// CHECK: This account use to satisfy accounts context. The pool only has fee in token b account
    #[account(mut)]
    pub token_a_account: UncheckedAccount<'info>,

    /// The token b account
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
    /// CHECK: dbc event authority
    pub dbc_event_authority: UncheckedAccount<'info>,

    /// signer
    pub signer: Signer<'info>,
}

pub fn handle_funding_by_claim_dbc_creator_trading_fee(
    ctx: Context<FundingByClaimDbcCreatorTradingFeeCtx>,
) -> Result<()> {
    let config = ctx.accounts.config.load()?;
    // support collect fee mode is 0 (only quote token)
    require!(config.collect_fee_mode == 0, FeeVaultError::InvalidDbcPool);

    handle_funding_fee(
        ctx.accounts.signer.key,
        &ctx.accounts.fee_vault,
        &mut ctx.accounts.token_b_account.clone(),
        ctx.accounts.quote_mint.key,
        ctx.accounts.pool.key(),
        FundingType::ClaimDbcCreatorTradingFee,
        |signer_seeds| {
            dynamic_bonding_curve::cpi::claim_creator_trading_fee(
                CpiContext::new_with_signer(
                    ctx.accounts.dbc_program.to_account_info(),
                    dynamic_bonding_curve::cpi::accounts::ClaimCreatorTradingFee {
                        pool_authority: ctx.accounts.dbc_pool_authority.to_account_info(),
                        pool: ctx.accounts.pool.to_account_info(),
                        token_a_account: ctx.accounts.token_a_account.to_account_info(),
                        token_b_account: ctx.accounts.token_b_account.to_account_info(),
                        base_vault: ctx.accounts.base_vault.to_account_info(),
                        quote_vault: ctx.accounts.quote_vault.to_account_info(),
                        base_mint: ctx.accounts.base_mint.to_account_info(),
                        quote_mint: ctx.accounts.quote_mint.to_account_info(),
                        creator: ctx.accounts.fee_vault.to_account_info(),
                        token_base_program: ctx.accounts.token_base_program.to_account_info(),
                        token_quote_program: ctx.accounts.token_quote_program.to_account_info(),
                        event_authority: ctx.accounts.dbc_event_authority.to_account_info(),
                        program: ctx.accounts.dbc_program.to_account_info(),
                    },
                    &[signer_seeds],
                ),
                0,        // max base amount,
                u64::MAX, // max quote amount,
            )?;
            Ok(())
        },
    )?;

    Ok(())
}

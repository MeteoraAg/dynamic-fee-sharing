use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use damm_v2::accounts::Pool;

use crate::{
    error::FeeVaultError,
    event::EvtFundFee,
    math::SafeMath,
    state::{FeeVault, FundingType},
};

#[event_cpi]
#[derive(Accounts)]
pub struct FundingByClaimDammv2FeeCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    pub pool: AccountLoader<'info, Pool>,

    /// CHECK: pool
    #[account(mut)]
    pub position: UncheckedAccount<'info>,

    /// CHECK:
    /// The token account for nft
    pub position_nft_account: UncheckedAccount<'info>,

    /// CHECK: This account use to satisfy accounts context. The pool only has fee in token b account
    #[account(mut)]
    pub token_a_account: UncheckedAccount<'info>,

    /// The user token b account
    #[account(mut)]
    pub token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK:
    /// The vault token account for input token
    #[account(mut)]
    pub token_a_vault: UncheckedAccount<'info>,

    /// CHECK:
    /// The vault token account for output token
    #[account(mut)]
    pub token_b_vault: UncheckedAccount<'info>,

    /// CHECK:
    /// The mint of token a
    pub token_a_mint: UncheckedAccount<'info>,

    /// CHECK:
    /// The mint of token b
    pub token_b_mint: UncheckedAccount<'info>,

    /// CHECK: Token a program
    pub token_a_program: UncheckedAccount<'info>,

    /// CHECK: Token b program
    pub token_b_program: UncheckedAccount<'info>,

    /// CHECK: dammv2 pool authority
    pub dammv2_pool_authority: UncheckedAccount<'info>,

    /// CHECK: dammv2 program
    #[account(address = damm_v2::ID)]
    pub dammv2_program: UncheckedAccount<'info>,
    /// CHECK: dammv2 authority
    pub dammv2_event_authority: UncheckedAccount<'info>,
}

pub fn handle_funding_by_claim_dammv2_fee(ctx: Context<FundingByClaimDammv2FeeCtx>) -> Result<()> {
    let pool = ctx.accounts.pool.load()?;
    // support collect fee mode is 1 (only token B)
    require!(pool.collect_fee_mode == 1, FeeVaultError::InvalidDammv2Pool);

    let fee_vault = ctx.accounts.fee_vault.load()?;

    require!(
        fee_vault
            .token_vault
            .eq(&ctx.accounts.token_b_account.key())
            && fee_vault.token_mint.eq(&ctx.accounts.token_b_mint.key()),
        FeeVaultError::InvalidFeeVault
    );

    // support fee vault type is pda account
    require!(
        fee_vault.fee_vault_type == 0,
        FeeVaultError::InvalidFeeVault
    );

    let before_token_vault_balance = ctx.accounts.token_b_account.amount;

    let signer_seeds = fee_vault_seeds!(
        fee_vault.base,
        fee_vault.token_mint,
        fee_vault.fee_vault_bump
    );

    damm_v2::cpi::claim_position_fee(CpiContext::new_with_signer(
        ctx.accounts.dammv2_program.to_account_info(),
        damm_v2::cpi::accounts::ClaimPositionFee {
            pool_authority: ctx.accounts.dammv2_pool_authority.to_account_info(),
            pool: ctx.accounts.pool.to_account_info(),
            position: ctx.accounts.position.to_account_info(),
            token_a_account: ctx.accounts.token_a_account.to_account_info(),
            token_b_account: ctx.accounts.token_b_account.to_account_info(),
            token_a_vault: ctx.accounts.token_a_vault.to_account_info(),
            token_b_vault: ctx.accounts.token_b_vault.to_account_info(),
            token_a_program: ctx.accounts.token_a_program.to_account_info(),
            token_b_program: ctx.accounts.token_b_program.to_account_info(),
            token_a_mint: ctx.accounts.token_a_mint.to_account_info(),
            token_b_mint: ctx.accounts.token_b_mint.to_account_info(),
            position_nft_account: ctx.accounts.position_nft_account.to_account_info(),
            owner: ctx.accounts.fee_vault.to_account_info(),
            event_authority: ctx.accounts.dammv2_event_authority.to_account_info(),
            program: ctx.accounts.dammv2_program.to_account_info(),
        },
        &[signer_seeds],
    ))?;

    ctx.accounts.token_b_account.reload()?;

    let after_token_vault_balance = ctx.accounts.token_b_account.amount;

    let claimed_amount = after_token_vault_balance.safe_sub(before_token_vault_balance)?;

    drop(fee_vault);

    if claimed_amount > 0 {
        let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
        fee_vault.fund_fee(claimed_amount)?;

        emit_cpi!(EvtFundFee {
            funding_type: FundingType::ClaimDammV2,
            fee_vault: ctx.accounts.fee_vault.key(),
            funder: ctx.accounts.pool.key(),
            funded_amount: claimed_amount,
            fee_per_share: fee_vault.fee_per_share,
        });
    }

    Ok(())
}

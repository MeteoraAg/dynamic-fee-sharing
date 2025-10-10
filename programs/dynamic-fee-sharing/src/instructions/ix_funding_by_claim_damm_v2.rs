use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use damm_v2::accounts::Pool;

use crate::{
    error::FeeVaultError,
    handle_funding_fee,
    state::{FeeVault, FundingType},
};

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

    /// signer
    pub signer: Signer<'info>,
}

pub fn handle_funding_by_claim_dammv2_fee(ctx: Context<FundingByClaimDammv2FeeCtx>) -> Result<()> {
    let pool = ctx.accounts.pool.load()?;
    // support collect fee mode is 1 (only token B)
    require!(pool.collect_fee_mode == 1, FeeVaultError::InvalidDammv2Pool);

    handle_funding_fee(
        ctx.accounts.signer.key,
        &ctx.accounts.fee_vault,
        &mut ctx.accounts.token_b_account.clone(),
        ctx.accounts.token_b_mint.key,
        ctx.accounts.pool.key(),
        FundingType::ClaimDammV2,
        |signer_seeds| {
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

            Ok(())
        },
    )?;

    Ok(())
}

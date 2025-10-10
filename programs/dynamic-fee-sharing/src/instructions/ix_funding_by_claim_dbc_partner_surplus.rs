use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use dynamic_bonding_curve::accounts::VirtualPool;

use crate::{
    handle_funding_fee,
    state::{FeeVault, FundingType},
};

#[derive(Accounts)]
pub struct FundingByClaimDbcPartnerSurplusCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: The pool config
    pub config: UncheckedAccount<'info>,

    /// The dbc virtual pool
    #[account(mut)]
    pub pool: AccountLoader<'info, VirtualPool>,

    /// The token b account
    #[account(mut)]
    pub token_quote_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: The vault token account for quote token
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,

    /// CHECK: The mint of token base
    pub quote_mint: UncheckedAccount<'info>,

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

pub fn handle_funding_by_claim_dbc_partner_surplus(
    ctx: Context<FundingByClaimDbcPartnerSurplusCtx>,
) -> Result<()> {
    let virtual_pool = ctx.accounts.pool.load()?;
    // partner surplus has been withdraw
    if virtual_pool.is_partner_withdraw_surplus == 1 {
        return Ok(());
    }

    drop(virtual_pool);

    handle_funding_fee(
        ctx.accounts.signer.key,
        &ctx.accounts.fee_vault,
        &mut ctx.accounts.token_quote_account.clone(),
        ctx.accounts.quote_mint.key,
        ctx.accounts.pool.key(),
        FundingType::ClaimDbcPartnerSurplus,
        |signer_seeds| {
            dynamic_bonding_curve::cpi::partner_withdraw_surplus(CpiContext::new_with_signer(
                ctx.accounts.dbc_program.to_account_info(),
                dynamic_bonding_curve::cpi::accounts::PartnerWithdrawSurplus {
                    pool_authority: ctx.accounts.dbc_pool_authority.to_account_info(),
                    config: ctx.accounts.config.to_account_info(),
                    virtual_pool: ctx.accounts.pool.to_account_info(),
                    token_quote_account: ctx.accounts.token_quote_account.to_account_info(),
                    quote_vault: ctx.accounts.quote_vault.to_account_info(),
                    quote_mint: ctx.accounts.quote_mint.to_account_info(),
                    fee_claimer: ctx.accounts.fee_vault.to_account_info(),
                    token_quote_program: ctx.accounts.token_quote_program.to_account_info(),
                    event_authority: ctx.accounts.dbc_event_authority.to_account_info(),
                    program: ctx.accounts.dbc_program.to_account_info(),
                },
                &[signer_seeds],
            ))?;
            Ok(())
        },
    )?;

    Ok(())
}

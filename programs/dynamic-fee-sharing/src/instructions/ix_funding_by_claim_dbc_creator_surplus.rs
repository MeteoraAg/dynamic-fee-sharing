use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use dynamic_bonding_curve::accounts::VirtualPool;

use crate::{handle_funding_fee, state::FeeVault};

#[event_cpi]
#[derive(Accounts)]
pub struct FundingByClaimDbcCreatorSurplusCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: The pool config
    pub config: UncheckedAccount<'info>,

    /// The dbc virtual pool
    #[account(mut)]
    pub pool: AccountLoader<'info, VirtualPool>,

    /// The treasury token b account
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

pub fn handle_funding_by_claim_dbc_creator_surplus(
    ctx: Context<FundingByClaimDbcCreatorSurplusCtx>,
) -> Result<()> {
    let virtual_pool = ctx.accounts.pool.load()?;
    // creator surplus has been withdraw
    if virtual_pool.is_creator_withdraw_surplus == 1 {
        return Ok(());
    }

    drop(virtual_pool);

    handle_funding_fee(
        ctx.accounts.signer.key,
        &ctx.accounts.fee_vault,
        &mut ctx.accounts.token_quote_account.clone(),
        ctx.accounts.quote_mint.key,
        |signer_seeds| {
            dynamic_bonding_curve::cpi::creator_withdraw_surplus(CpiContext::new_with_signer(
                ctx.accounts.dbc_program.to_account_info(),
                dynamic_bonding_curve::cpi::accounts::CreatorWithdrawSurplus {
                    pool_authority: ctx.accounts.dbc_pool_authority.to_account_info(),
                    config: ctx.accounts.config.to_account_info(),
                    virtual_pool: ctx.accounts.pool.to_account_info(),
                    token_quote_account: ctx.accounts.token_quote_account.to_account_info(),
                    quote_vault: ctx.accounts.quote_vault.to_account_info(),
                    quote_mint: ctx.accounts.quote_mint.to_account_info(),
                    creator: ctx.accounts.fee_vault.to_account_info(),
                    token_quote_program: ctx.accounts.token_quote_program.to_account_info(),
                    event_authority: ctx.accounts.dbc_event_authority.to_account_info(),
                    program: ctx.accounts.dbc_program.to_account_info(),
                },
                &[signer_seeds],
            ))?;
            Ok(())
        },
    )?;

    // emit_cpi!(EvtFundFee {
    //     funding_type: FundingType::ClaimDbcCreatorSurplus,
    //     fee_vault: ctx.accounts.fee_vault.key(),
    //     funder: ctx.accounts.pool.key(),
    //     funded_amount: claimed_amount,
    //     fee_per_share: fee_vault.fee_per_share,
    // });

    Ok(())
}

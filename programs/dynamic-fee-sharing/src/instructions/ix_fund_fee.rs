use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::FeeVaultError;
use crate::event::EvtFundFee;
use crate::state::{FeeVault, FundingType};
use crate::utils::token::{calculate_transfer_fee_excluded_amount, transfer_from_user};

#[event_cpi]
#[derive(Accounts)]
pub struct FundFeeCtx<'info> {
    #[account(mut, has_one = token_vault, has_one = token_mint)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub fund_token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub funder: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_fund_fee(ctx: Context<FundFeeCtx>, max_amount: u64) -> Result<()> {
    let amount = max_amount.min(ctx.accounts.fund_token_vault.amount);
    require!(amount > 0, FeeVaultError::AmountIsZero);

    // transfer token
    let excluded_transfer_fee_amount =
        calculate_transfer_fee_excluded_amount(&ctx.accounts.token_mint, amount)?.amount;

    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
    fee_vault.fund_fee(excluded_transfer_fee_amount)?;

    transfer_from_user(
        &ctx.accounts.funder,
        &ctx.accounts.token_mint,
        &ctx.accounts.fund_token_vault,
        &ctx.accounts.token_vault,
        &ctx.accounts.token_program,
        amount,
    )?;

    emit_cpi!(EvtFundFee {
        funding_type: FundingType::Direct,
        fee_vault: ctx.accounts.fee_vault.key(),
        funder: ctx.accounts.funder.key(),
        funded_amount: excluded_transfer_fee_amount,
        fee_per_share: fee_vault.fee_per_share
    });

    Ok(())
}

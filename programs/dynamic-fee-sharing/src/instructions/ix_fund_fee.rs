use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::FeeVaultError;
use crate::event::EvtFundFee;
use crate::utils::load_vault_mut;
use crate::utils::token::{calculate_transfer_fee_excluded_amount, transfer_from_user};

#[event_cpi]
#[derive(Accounts)]
pub struct FundFeeCtx<'info> {
    /// CHECK: FeeVault or DynamicFeeVault
    #[account(mut)]
    pub fee_vault: UncheckedAccount<'info>,

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

    let fee_vault_info = ctx.accounts.fee_vault.to_account_info();
    let mut vault = load_vault_mut(&fee_vault_info)?;

    let is_token_0 = vault.validate_token_accounts(
        &ctx.accounts.token_vault.key(),
        &ctx.accounts.token_mint.key(),
    )?;
    let fee_per_share = vault.fund_fee(is_token_0, excluded_transfer_fee_amount)?;
    drop(vault);

    transfer_from_user(
        &ctx.accounts.funder,
        &ctx.accounts.token_mint,
        &ctx.accounts.fund_token_vault,
        &ctx.accounts.token_vault,
        &ctx.accounts.token_program.to_account_info(),
        amount,
    )?;

    emit_cpi!(EvtFundFee {
        source_program: Pubkey::default(),
        fee_vault: ctx.accounts.fee_vault.key(),
        payload: vec![],
        funded_amount: excluded_transfer_fee_amount,
        fee_per_share,
    });

    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;
use dynamic_bonding_curve::accounts::VirtualPool;

use crate::{
    error::FeeVaultError,
    event::EvtFundFee,
    math::SafeMath,
    state::{FeeVault, FundingType},
};

#[event_cpi]
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
}

pub fn handle_funding_by_claim_dbc_partner_surplus(
    ctx: Context<FundingByClaimDbcPartnerSurplusCtx>,
) -> Result<()> {
    let fee_vault = ctx.accounts.fee_vault.load()?;
    let virtual_pool = ctx.accounts.pool.load()?;

    // partner surplus has been withdraw
    if virtual_pool.is_partner_withdraw_surplus == 1 {
        return Ok(());
    }

    drop(virtual_pool);

    require!(
        fee_vault
            .token_vault
            .eq(&ctx.accounts.token_quote_account.key())
            && fee_vault.token_mint.eq(&ctx.accounts.quote_mint.key()),
        FeeVaultError::InvalidFeeVault
    );

    // support fee vault type is pda account
    require!(
        fee_vault.fee_vault_type == 1,
        FeeVaultError::InvalidFeeVault
    );

    let before_token_vault_balance = ctx.accounts.token_quote_account.amount;

    let signer_seeds = fee_vault_seeds!(
        fee_vault.base,
        fee_vault.token_mint,
        fee_vault.fee_vault_bump
    );

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
    ctx.accounts.token_quote_account.reload()?;

    let after_token_vault_balance = ctx.accounts.token_quote_account.amount;

    let claimed_amount = after_token_vault_balance.safe_sub(before_token_vault_balance)?;

    drop(fee_vault);

    if claimed_amount > 0 {
        let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
        fee_vault.fund_fee(claimed_amount)?;

        emit_cpi!(EvtFundFee {
            funding_type: FundingType::ClaimDbcPartnerSurplus,
            fee_vault: ctx.accounts.fee_vault.key(),
            funder: ctx.accounts.pool.key(),
            funded_amount: claimed_amount,
            fee_per_share: fee_vault.fee_per_share,
        });
    }

    Ok(())
}

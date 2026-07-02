use crate::const_pda;
use crate::constants::seeds::USER_UNCLAIMED_FEE_PREFIX;
use crate::error::FeeVaultError;
use crate::event::EvtClaimUnclaimedFee;
use crate::state::{DynamicFeeVault, UserUnclaimedFee};
use crate::utils::token::transfer_from_fee_vault;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
pub struct ClaimUnclaimedFeeCtx<'info> {
    #[account(has_one = token_0_mint, has_one = owner, has_one = token_0_vault)]
    pub fee_vault: AccountLoader<'info, DynamicFeeVault>,

    /// CHECK: fee vault authority
    #[account(address = const_pda::fee_vault_authority::ID)]
    pub fee_vault_authority: UncheckedAccount<'info>,

    pub token_0_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub token_0_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // token account does not need to be owned by user
    #[account(mut)]
    pub user_token_0_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// optional second token slot; required when the user has unclaimed fee there,
    /// validated against the fee vault header in the handler
    pub token_1_mint: Option<Box<InterfaceAccount<'info, Mint>>>,

    #[account(mut)]
    pub token_1_vault: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(mut)]
    pub user_token_1_vault: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(
        mut,
        close = owner,
        seeds = [
            USER_UNCLAIMED_FEE_PREFIX,
            fee_vault.key().as_ref(),
            user.key().as_ref(),
        ],
        bump,
    )]
    pub user_unclaimed_fee: AccountLoader<'info, UserUnclaimedFee>,

    /// CHECK: fee vault owner, receives rent from closed account
    #[account(mut)]
    pub owner: UncheckedAccount<'info>,

    pub user: Signer<'info>,

    pub token_0_program: Interface<'info, TokenInterface>,

    pub token_1_program: Option<Interface<'info, TokenInterface>>,
}

fn validate_token_1_accounts<'a, 'info>(
    accounts: &'a ClaimUnclaimedFeeCtx<'info>,
) -> Result<(
    &'a InterfaceAccount<'info, Mint>,
    &'a InterfaceAccount<'info, TokenAccount>,
    &'a InterfaceAccount<'info, TokenAccount>,
    &'a Interface<'info, TokenInterface>,
)> {
    let token_1_mint = accounts
        .token_1_mint
        .as_deref()
        .ok_or_else(|| error!(FeeVaultError::InvalidFeeVault))?;
    let token_1_vault = accounts
        .token_1_vault
        .as_deref()
        .ok_or_else(|| error!(FeeVaultError::InvalidFeeVault))?;
    let user_token_1_vault = accounts
        .user_token_1_vault
        .as_deref()
        .ok_or_else(|| error!(FeeVaultError::InvalidFeeVault))?;
    let token_1_program = accounts
        .token_1_program
        .as_ref()
        .ok_or_else(|| error!(FeeVaultError::InvalidFeeVault))?;

    let fee_vault = accounts.fee_vault.load()?;
    require!(
        fee_vault.token_1_mint.eq(&token_1_mint.key())
            && fee_vault.token_1_vault.eq(&token_1_vault.key()),
        FeeVaultError::InvalidFeeVault
    );

    Ok((
        token_1_mint,
        token_1_vault,
        user_token_1_vault,
        token_1_program,
    ))
}

pub fn handle_claim_unclaimed_fee(ctx: Context<ClaimUnclaimedFeeCtx>) -> Result<()> {
    let user_unclaimed_fee = ctx.accounts.user_unclaimed_fee.load()?;
    let claimed_fee_0 = user_unclaimed_fee.unclaimed_fee_0;
    let claimed_fee_1 = user_unclaimed_fee.unclaimed_fee_1;
    drop(user_unclaimed_fee);

    if claimed_fee_0 > 0 {
        transfer_from_fee_vault(
            ctx.accounts.fee_vault_authority.to_account_info(),
            &ctx.accounts.token_0_mint,
            &ctx.accounts.token_0_vault,
            &ctx.accounts.user_token_0_vault,
            &ctx.accounts.token_0_program.to_account_info(),
            claimed_fee_0,
        )?;
    }

    if claimed_fee_1 > 0 {
        let (token_1_mint, token_1_vault, user_token_1_vault, token_1_program) =
            validate_token_1_accounts(&ctx.accounts)?;

        transfer_from_fee_vault(
            ctx.accounts.fee_vault_authority.to_account_info(),
            token_1_mint,
            token_1_vault,
            user_token_1_vault,
            &token_1_program.to_account_info(),
            claimed_fee_1,
        )?;
    }

    if claimed_fee_0 > 0 || claimed_fee_1 > 0 {
        emit_cpi!(EvtClaimUnclaimedFee {
            fee_vault: ctx.accounts.fee_vault.key(),
            user: ctx.accounts.user.key(),
            claimed_fee_0,
            claimed_fee_1,
        });
    }

    Ok(())
}

use crate::const_pda;
use crate::constants::seeds::USER_UNCLAIMED_FEE_PREFIX;
use crate::event::EvtClaimUnclaimedFee;
use crate::state::{FeeVault, UserUnclaimedFee};
use crate::utils::token::transfer_from_fee_vault;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
pub struct ClaimUnclaimedFeeCtx<'info> {
    #[account(has_one = token_mint, has_one = owner, has_one = token_vault)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: fee vault authority
    #[account(address = const_pda::fee_vault_authority::ID)]
    pub fee_vault_authority: UncheckedAccount<'info>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

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

    // token account does not need to be owned by user
    #[account(mut)]
    pub user_token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: fee vault owner, receives rent from closed account
    #[account(mut)]
    pub owner: UncheckedAccount<'info>,

    pub user: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

// when a user is removed from a fee vault, they may claim any unclaimed fee that they have earned before the removal
pub fn handle_claim_unclaimed_fee(ctx: Context<ClaimUnclaimedFeeCtx>) -> Result<()> {
    let user_unclaimed_fee = ctx.accounts.user_unclaimed_fee.load()?;
    let fee_being_claimed = user_unclaimed_fee.unclaimed_fee;

    if fee_being_claimed > 0 {
        transfer_from_fee_vault(
            ctx.accounts.fee_vault_authority.to_account_info(),
            &ctx.accounts.token_mint,
            ctx.accounts.token_vault.to_account_info(),
            ctx.accounts.user_token_vault.to_account_info(),
            &ctx.accounts.token_program,
            fee_being_claimed,
        )?;
    }

    emit_cpi!(EvtClaimUnclaimedFee {
        fee_vault: ctx.accounts.fee_vault.key(),
        user: ctx.accounts.user.key(),
        claimed_fee: fee_being_claimed,
    });

    Ok(())
}

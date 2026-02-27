use crate::const_pda;
use crate::constants::seeds::REMOVED_USER_TOKEN_VAULT;
use crate::event::EvtClaimRemovedUserFee;
use crate::state::FeeVault;
use crate::utils::token::transfer_from_fee_vault;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    close_account, CloseAccount, Mint, TokenAccount, TokenInterface,
};

#[event_cpi]
#[derive(Accounts)]
pub struct ClaimRemovedUserFeeCtx<'info> {
    #[account(has_one = token_mint, has_one = owner)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: fee vault authority
    #[account(address = const_pda::fee_vault_authority::ID)]
    pub fee_vault_authority: UncheckedAccount<'info>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        seeds = [
            REMOVED_USER_TOKEN_VAULT,
            fee_vault.key().as_ref(),
            token_mint.key().as_ref(),
            user.key().as_ref(),
        ],
        bump,
        token::mint = token_mint,
        token::authority = fee_vault_authority,
    )]
    pub removed_user_token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut, 
        token::authority = user, 
        token::mint = token_mint,
    )]
    pub user_token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: fee vault owner, receives rent from closed account
    #[account(mut)]
    pub owner: UncheckedAccount<'info>,

    pub user: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_claim_removed_user_fee(ctx: Context<ClaimRemovedUserFeeCtx>) -> Result<()> {
    let fee_being_claimed = ctx.accounts.removed_user_token_vault.amount;

    if fee_being_claimed > 0 {
        transfer_from_fee_vault(
            ctx.accounts.fee_vault_authority.to_account_info(),
            &ctx.accounts.token_mint,
            ctx.accounts.removed_user_token_vault.to_account_info(),
            ctx.accounts.user_token_vault.to_account_info(),
            &ctx.accounts.token_program,
            fee_being_claimed,
        )?;
    }

    let signer_seeds = fee_vault_authority_seeds!();
    close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.removed_user_token_vault.to_account_info(),
            destination: ctx.accounts.owner.to_account_info(),
            authority: ctx.accounts.fee_vault_authority.to_account_info(),
        },
        &[&signer_seeds[..]],
    ))?;

    emit_cpi!(EvtClaimRemovedUserFee {
        fee_vault: ctx.accounts.fee_vault.key(),
        user: ctx.accounts.user.key(),
        claimed_fee: fee_being_claimed,
    });

    Ok(())
}

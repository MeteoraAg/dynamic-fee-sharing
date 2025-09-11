use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::const_pda;
use crate::event::EvtClaimFee;
use crate::state::FeeVault;
use crate::utils::token::transfer_from_fee_vault;

#[event_cpi]
#[derive(Accounts)]
pub struct ClaimFeeCtx<'info> {
    /// CHECK: fee vault authority
    #[account(
        address = const_pda::fee_vault_authority::ID
    )]
    pub fee_vault_authority: UncheckedAccount<'info>,

    #[account(mut, has_one = token_vault, has_one = token_mint)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub user_token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub user: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_claim_fee(ctx: Context<ClaimFeeCtx>, index: u8) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
    let fee_being_claimed = fee_vault.validate_and_claim_fee(index, &ctx.accounts.user.key())?;

    if fee_being_claimed > 0 {
        transfer_from_fee_vault(
            ctx.accounts.fee_vault_authority.to_account_info(),
            &ctx.accounts.token_mint,
            &ctx.accounts.token_vault,
            &ctx.accounts.user_token_vault,
            &ctx.accounts.token_program,
            fee_being_claimed,
        )?;

        emit_cpi!(EvtClaimFee {
            fee_vault: ctx.accounts.fee_vault.key(),
            index,
            user: ctx.accounts.user.key(),
            claimed_fee: fee_being_claimed,
        });
    }

    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::const_pda;
use crate::event::EvtClaimFee;
use crate::state::VaultOps;
use crate::utils::load_vault_mut;
use crate::utils::token::transfer_from_fee_vault;

#[event_cpi]
#[derive(Accounts)]
pub struct ClaimFeeCtx<'info> {
    /// CHECK: FeeVault or DynamicFeeVault
    #[account(mut)]
    pub fee_vault: UncheckedAccount<'info>,

    /// CHECK: fee vault authority
    #[account(
        address = const_pda::fee_vault_authority::ID
    )]
    pub fee_vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub user_token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub user: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_claim_fee(ctx: Context<ClaimFeeCtx>, index: u8) -> Result<()> {
    let fee_vault_info = ctx.accounts.fee_vault.to_account_info();
    let mut fee_vault = load_vault_mut(&fee_vault_info)?;

    fee_vault.validate_token_accounts(
        &ctx.accounts.token_vault.key(),
        &ctx.accounts.token_mint.key(),
    )?;

    let fee_being_claimed = fee_vault.validate_and_claim_fee(index, &ctx.accounts.user.key())?;
    drop(fee_vault);

    if fee_being_claimed > 0 {
        transfer_from_fee_vault(
            ctx.accounts.fee_vault_authority.to_account_info(),
            &ctx.accounts.token_mint,
            &ctx.accounts.token_vault,
            &ctx.accounts.user_token_vault,
            &ctx.accounts.token_program.to_account_info(),
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

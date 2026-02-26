use crate::const_pda;
use crate::constants::seeds::REMOVED_USER_TOKEN_VAULT;
use crate::event::EvtRemoveUser;
use crate::state::FeeVault;
use crate::utils::token::transfer_from_fee_vault;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
pub struct RemoveUserCtx<'info> {
    #[account(mut, has_one = token_vault, has_one = token_mint)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: fee vault authority
    #[account(address = const_pda::fee_vault_authority::ID)]
    pub fee_vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: the user being removed
    pub user: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = signer,
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

    #[account(mut)]
    pub signer: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handle_remove_user(ctx: Context<RemoveUserCtx>) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
    let user = ctx.accounts.user.key();
    let unclaimed_fee = fee_vault.validate_and_remove_user_and_get_unclaimed_fee(&user)?;

    if unclaimed_fee > 0 {
        transfer_from_fee_vault(
            ctx.accounts.fee_vault_authority.to_account_info(),
            &ctx.accounts.token_mint,
            &ctx.accounts.token_vault,
            &ctx.accounts.removed_user_token_vault,
            &ctx.accounts.token_program,
            unclaimed_fee,
        )?;
    }

    emit_cpi!(EvtRemoveUser {
        fee_vault: ctx.accounts.fee_vault.key(),
        user,
        unclaimed_fee,
    });

    Ok(())
}

use crate::const_pda;
use crate::constants::seeds::REMOVED_USER_TOKEN_VAULT;
use crate::event::EvtRemoveUser;
use crate::state::FeeVault;
use crate::utils::token::{create_pda_token_account, transfer_from_fee_vault};
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

    /// CHECK: PDA token vault for removed user's unclaimed fees. Created in handler only when unclaimed_fee > 0.
    #[account(
        mut,
        seeds = [
            REMOVED_USER_TOKEN_VAULT,
            fee_vault.key().as_ref(),
            token_mint.key().as_ref(),
            user.key().as_ref(),
        ],
        bump,
    )]
    pub removed_user_token_vault: UncheckedAccount<'info>,

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
        let removed_user_token_vault = &ctx.accounts.removed_user_token_vault;

        if removed_user_token_vault.data_is_empty() {
            let fee_vault_key = ctx.accounts.fee_vault.key();
            let token_mint_key = ctx.accounts.token_mint.key();
            let bump = ctx.bumps.removed_user_token_vault;

            create_pda_token_account(
                ctx.accounts.signer.to_account_info(),
                removed_user_token_vault.to_account_info(),
                &ctx.accounts.token_mint,
                &ctx.accounts.fee_vault_authority.key(),
                &ctx.accounts.token_program,
                ctx.accounts.system_program.to_account_info(),
                &[
                    REMOVED_USER_TOKEN_VAULT,
                    fee_vault_key.as_ref(),
                    token_mint_key.as_ref(),
                    user.as_ref(),
                    &[bump],
                ],
            )?;
        }

        transfer_from_fee_vault(
            ctx.accounts.fee_vault_authority.to_account_info(),
            &ctx.accounts.token_mint,
            ctx.accounts.token_vault.to_account_info(),
            removed_user_token_vault.to_account_info(),
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

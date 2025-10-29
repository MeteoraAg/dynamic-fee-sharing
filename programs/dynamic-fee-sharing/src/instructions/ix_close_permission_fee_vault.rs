use crate::{
    constants::seeds::FEE_VAULT_AUTHORITY_PREFIX, 
    event::EvtCloseFeeVault, 
    state::FeeVault, 
    utils::token::transfer_from_fee_vault
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
pub struct ClosePermissionFeeVaultCtx<'info> {
    /// CHECK: pool authority
    #[account(
            seeds = [
                FEE_VAULT_AUTHORITY_PREFIX.as_ref(),
            ],
            bump,
        )]
    pub fee_vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = admin,
        has_one = token_vault,
        has_one = token_mint, 
        close = rent_receiver
    )]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mint::token_program = token_program,
    )]
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: rent fee receiver
    #[account(mut)]
    pub rent_receiver: UncheckedAccount<'info>,

    /// CHECK: remaining fee receiver
    #[account(mut)]
    pub fee_receiver: Box<InterfaceAccount<'info, TokenAccount>>,

    pub admin: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_close_permission_fee_vault(ctx: Context<ClosePermissionFeeVaultCtx>) -> Result<()> {
    let remaining_fee = ctx.accounts.token_vault.amount;
    if remaining_fee > 0 {
        transfer_from_fee_vault(
            ctx.accounts.fee_vault_authority.to_account_info(),
            &ctx.accounts.token_mint,
            &ctx.accounts.token_vault,
            &ctx.accounts.fee_receiver,
            &ctx.accounts.token_program,
            remaining_fee,
        )?;

        emit_cpi!(EvtCloseFeeVault{
            fee_vault: ctx.accounts.fee_vault.key(),
            admin: ctx.accounts.admin.key(),
            fee_receiver: ctx.accounts.fee_receiver.key(),
            rent_receiver: ctx.accounts.rent_receiver.key(),
            remaining_fee
        })
    }

    Ok(())
}

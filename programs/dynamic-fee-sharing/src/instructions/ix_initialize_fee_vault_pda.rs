use crate::constants::seeds::FEE_VAULT_PREFIX;
use crate::event::EvtInitializeFeeVault;
use crate::state::FeeVaultType;
use crate::{
    constants::seeds::{FEE_VAULT_AUTHORITY_PREFIX, TOKEN_VAULT_PREFIX},
    state::FeeVault,
};
use crate::{create_fee_vault, InitializeFeeVaultParameters};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
pub struct InitializeFeeVaultPdaCtx<'info> {
    #[account(
        init,
        seeds = [
            FEE_VAULT_PREFIX.as_ref(),
            base.key().as_ref(),
            token_mint.key().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + FeeVault::INIT_SPACE
    )]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// CHECK: pool authority
    #[account(
            seeds = [
                FEE_VAULT_AUTHORITY_PREFIX.as_ref(),
            ],
            bump,
        )]
    pub fee_vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [
            TOKEN_VAULT_PREFIX.as_ref(),
            fee_vault.key().as_ref(),
        ],
        token::mint = token_mint,
        token::authority = fee_vault_authority,
        token::token_program = token_program,
        payer = payer,
        bump,
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mint::token_program = token_program,
    )]
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: owner
    pub owner: UncheckedAccount<'info>,

    pub base: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    // Sysvar for program account
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_fee_vault_pda(
    ctx: Context<InitializeFeeVaultPdaCtx>,
    params: &InitializeFeeVaultParameters,
) -> Result<()> {
    create_fee_vault(
        &ctx.accounts.token_mint,
        params,
        &ctx.accounts.fee_vault,
        ctx.accounts.owner.key,
        &ctx.accounts.token_vault.key(),
        &ctx.accounts.base.key,
        ctx.bumps.fee_vault,
        FeeVaultType::PdaAccount.into(),
    )?;

    emit_cpi!(EvtInitializeFeeVault {
        fee_vault: ctx.accounts.fee_vault.key(),
        owner: ctx.accounts.owner.key(),
        token_mint: ctx.accounts.token_mint.key(),
        params: params.clone(),
        base: ctx.accounts.base.key(),
    });

    Ok(())
}

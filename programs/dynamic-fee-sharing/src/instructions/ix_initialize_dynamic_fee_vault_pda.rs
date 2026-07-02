use crate::constants::seeds::DYNAMIC_FEE_VAULT_PREFIX;
use crate::constants::seeds::{FEE_VAULT_AUTHORITY_PREFIX, TOKEN_VAULT_PREFIX};
use crate::create_dynamic_fee_vault;
use crate::event::EvtInitializeFeeVault;
use crate::params::InitializeFeeVaultParameters;
use crate::state::{DynamicFeeVault, VaultType};
use crate::utils::token::KeyOrDefault;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
#[instruction(params: InitializeFeeVaultParameters)]
pub struct InitializeDynamicFeeVaultPdaCtx<'info> {
    #[account(
        init,
        seeds = [
            DYNAMIC_FEE_VAULT_PREFIX.as_ref(),
            base.key().as_ref(),
            token_0_mint.key().as_ref(),
            token_1_mint.key_or_default().as_ref(),
        ],
        bump,
        payer = payer,
        space = DynamicFeeVault::space(params.users.len())
    )]
    pub fee_vault: AccountLoader<'info, DynamicFeeVault>,

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
        token::mint = token_0_mint,
        token::authority = fee_vault_authority,
        token::token_program = token_0_program,
        payer = payer,
        bump,
    )]
    pub token_0_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mint::token_program = token_0_program,
    )]
    pub token_0_mint: Box<InterfaceAccount<'info, Mint>>,

    /// optional second token mint slot
    #[account(
        mint::token_program = token_1_program,
    )]
    pub token_1_mint: Option<Box<InterfaceAccount<'info, Mint>>>,

    #[account(
        init,
        seeds = [
            TOKEN_VAULT_PREFIX.as_ref(),
            fee_vault.key().as_ref(),
            token_1_mint.key_or_default().as_ref(),
        ],
        token::mint = token_1_mint,
        token::authority = fee_vault_authority,
        token::token_program = token_1_program,
        payer = payer,
        bump,
    )]
    pub token_1_vault: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// CHECK: owner
    pub owner: UncheckedAccount<'info>,

    pub base: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_0_program: Interface<'info, TokenInterface>,

    pub token_1_program: Option<Interface<'info, TokenInterface>>,

    // Sysvar for program account
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_dynamic_fee_vault_pda(
    ctx: Context<InitializeDynamicFeeVaultPdaCtx>,
    params: &InitializeFeeVaultParameters,
) -> Result<()> {
    create_dynamic_fee_vault(
        &ctx.accounts.token_0_mint,
        ctx.accounts.token_1_mint.as_deref(),
        params,
        &ctx.accounts.fee_vault,
        ctx.accounts.owner.key,
        &ctx.accounts.token_0_vault.key(),
        ctx.accounts
            .token_1_vault
            .as_ref()
            .map(|token_1_vault| token_1_vault.key()),
        &ctx.accounts.base.key,
        ctx.bumps.fee_vault,
        VaultType::PdaAccount.into(),
    )?;

    emit_cpi!(EvtInitializeFeeVault {
        fee_vault: ctx.accounts.fee_vault.key(),
        owner: ctx.accounts.owner.key(),
        token_mint: ctx.accounts.token_0_mint.key(),
        params: params.clone(),
        base: ctx.accounts.base.key(),
    });

    Ok(())
}

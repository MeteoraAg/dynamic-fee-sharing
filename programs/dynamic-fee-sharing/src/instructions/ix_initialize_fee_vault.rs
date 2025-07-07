use crate::constants::MAX_USER;
use crate::error::FeeVaultError;
use crate::event::EvtInitializeFeeVault;
use crate::utils::token::{get_token_program_flags, is_supported_mint};
use crate::{
    constants::seeds::{FEE_VAULT_AUTHORITY_PREFIX, TOKEN_VAULT_PREFIX},
    state::FeeVault,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct InitializeFeeVaultParameters {
    pub padding: [u64; 8], // for future use
    pub users: Vec<UserShare>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
pub struct UserShare {
    pub address: Pubkey,
    pub share: u64,
}

impl InitializeFeeVaultParameters {
    pub fn validate(&self) -> Result<()> {
        let number_of_user = self.users.len();
        require!(
            number_of_user > 0 && number_of_user <= MAX_USER,
            FeeVaultError::InvalidFeeVaultParameters
        );
        for i in 0..number_of_user {
            require!(
                self.users[i].share > 0,
                FeeVaultError::InvalidFeeVaultParameters
            );
            require!(
                self.users[i].address.ne(&Pubkey::default()),
                FeeVaultError::InvalidFeeVaultParameters
            );
        }
        // that is fine to leave user addresses are duplicated?
        Ok(())
    }
}

#[event_cpi]
#[derive(Accounts)]
pub struct InitializeFeeVaultCtx<'info> {
    #[account(
        init,
        signer,
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

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    // Sysvar for program account
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_fee_vault(
    ctx: Context<InitializeFeeVaultCtx>,
    params: &InitializeFeeVaultParameters,
) -> Result<()> {
    create_fee_vault(
        &ctx.accounts.token_mint,
        params,
        &ctx.accounts.fee_vault,
        ctx.accounts.owner.key,
        &ctx.accounts.token_vault.key(),
    )?;

    emit_cpi!(EvtInitializeFeeVault {
        fee_vault: ctx.accounts.fee_vault.key(),
        owner: ctx.accounts.owner.key(),
        token_mint: ctx.accounts.token_mint.key(),
        params: params.clone(),
        base: Pubkey::default(),
    });

    Ok(())
}

pub fn create_fee_vault<'info>(
    token_mint: &Box<InterfaceAccount<'info, Mint>>,
    params: &InitializeFeeVaultParameters,
    fee_vault: &AccountLoader<'info, FeeVault>,
    owner: &Pubkey,
    token_vault: &Pubkey,
) -> Result<()> {
    require!(is_supported_mint(&token_mint)?, FeeVaultError::InvalidMint);

    params.validate()?;

    let mut fee_vault = fee_vault.load_init()?;
    fee_vault.initialize(
        owner,
        get_token_program_flags(&token_mint).into(),
        &token_mint.key(),
        token_vault,
        &params.users,
    )?;
    Ok(())
}

use crate::constants::seeds::{FEE_VAULT_AUTHORITY_PREFIX, TOKEN_VAULT_PREFIX};
use crate::constants::MAX_DYNAMIC_FEE_VAULT_USER;
use crate::error::FeeVaultError;
use crate::event::EvtInitializeFeeVault;
use crate::math::SafeMath;
use crate::params::InitializeFeeVaultParameters;
use crate::state::{DynamicFeeVault, DynamicUserFee, VaultType};
use crate::utils::d_load_mut_unchecked;
use crate::utils::token::{get_token_program_flags, is_supported_mint, KeyOrDefault};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
#[instruction(params: InitializeFeeVaultParameters)]
pub struct InitializeDynamicFeeVaultCtx<'info> {
    #[account(
        init,
        signer,
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

    /// optional second token slot; when absent the slot is disabled (Pubkey::default)
    #[account(
        mint::token_program = token_1_program,
    )]
    pub token_1_mint: Option<Box<InterfaceAccount<'info, Mint>>>,

    // mint in the seeds ties one vault per (fee_vault, mint);
    // slot 0 vault keeps [prefix, fee_vault] for FeeVault parity
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

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_0_program: Interface<'info, TokenInterface>,

    pub token_1_program: Option<Interface<'info, TokenInterface>>,

    // Sysvar for program account
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_dynamic_fee_vault(
    ctx: Context<InitializeDynamicFeeVaultCtx>,
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
        &Pubkey::default(),
        0,
        VaultType::NonPdaAccount.into(),
    )?;

    emit_cpi!(EvtInitializeFeeVault {
        fee_vault: ctx.accounts.fee_vault.key(),
        owner: ctx.accounts.owner.key(),
        token_mint: ctx.accounts.token_0_mint.key(),
        params: params.clone(),
        base: Pubkey::default(),
    });

    Ok(())
}

pub fn create_dynamic_fee_vault<'info>(
    token_0_mint: &InterfaceAccount<'info, Mint>,
    token_1_mint: Option<&InterfaceAccount<'info, Mint>>,
    params: &InitializeFeeVaultParameters,
    fee_vault: &AccountLoader<'info, DynamicFeeVault>,
    owner: &Pubkey,
    token_0_vault: &Pubkey,
    token_1_vault: Option<Pubkey>,
    base: &Pubkey,
    vault_bump: u8,
    vault_type: u8,
) -> Result<()> {
    require!(is_supported_mint(token_0_mint)?, FeeVaultError::InvalidMint);

    require!(
        token_1_mint.is_some() == token_1_vault.is_some(),
        FeeVaultError::InvalidFeeVaultParameters
    );

    let (token_1_flag, token_1_mint_key, token_1_vault_key) = match token_1_mint {
        Some(token_1_mint) => {
            require!(is_supported_mint(token_1_mint)?, FeeVaultError::InvalidMint);
            require!(
                token_1_mint.key().ne(&token_0_mint.key()),
                FeeVaultError::InvalidMint
            );

            (
                get_token_program_flags(token_1_mint).into(),
                token_1_mint.key(),
                token_1_vault.ok_or_else(|| error!(FeeVaultError::InvalidFeeVaultParameters))?,
            )
        }
        None => (0, Pubkey::default(), Pubkey::default()),
    };

    // TODO: determine reasonable amount of user for vault at creation. 100 won't fit in a tx
    params.validate(MAX_DYNAMIC_FEE_VAULT_USER, true)?;

    let mut vault = fee_vault.load_init()?;
    vault.initialize(
        owner,
        get_token_program_flags(token_0_mint).into(),
        &token_0_mint.key(),
        token_0_vault,
        token_1_flag,
        &token_1_mint_key,
        &token_1_vault_key,
        base,
        vault_bump,
        vault_type,
    );

    drop(vault); // drop before re-borrowing the account data

    let fee_vault_info = fee_vault.to_account_info();
    let mut vault = d_load_mut_unchecked::<DynamicFeeVault, DynamicUserFee>(&fee_vault_info)?;

    require!(
        vault.dynamic.len() == params.users.len(),
        FeeVaultError::InvalidFeeVaultParameters
    );

    let fee_per_share_token_0 = vault.fixed.fee_per_share_token_0;
    let fee_per_share_token_1 = vault.fixed.fee_per_share_token_1;
    let mut total_share = 0;
    for (i, user) in params.users.iter().enumerate() {
        vault.dynamic[i] = DynamicUserFee::new(
            user.address,
            user.share,
            fee_per_share_token_0,
            fee_per_share_token_1,
        );
        total_share = total_share.safe_add(user.share)?;
    }
    require!(total_share > 0, FeeVaultError::TotalShareIsZero);
    vault.fixed.total_share = total_share;

    Ok(())
}

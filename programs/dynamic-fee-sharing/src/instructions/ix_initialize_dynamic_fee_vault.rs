use crate::constants::seeds::{FEE_VAULT_AUTHORITY_PREFIX, TOKEN_VAULT_PREFIX};
use crate::constants::MAX_DYNAMIC_FEE_VAULT_USER;
use crate::error::FeeVaultError;
use crate::event::EvtInitializeFeeVault;
use crate::math::SafeMath;
use crate::params::InitializeFeeVaultParameters;
use crate::state::{DynamicFeeVault, UserFee, VaultType};
use crate::utils::d_load_mut_unchecked;
use crate::utils::token::{get_token_program_flags, is_supported_mint};
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

pub fn handle_initialize_dynamic_fee_vault(
    ctx: Context<InitializeDynamicFeeVaultCtx>,
    params: &InitializeFeeVaultParameters,
) -> Result<()> {
    create_dynamic_fee_vault(
        &ctx.accounts.token_mint,
        params,
        &ctx.accounts.fee_vault,
        ctx.accounts.owner.key,
        &ctx.accounts.token_vault.key(),
        &Pubkey::default(),
        0,
        VaultType::NonPdaAccount.into(),
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

pub fn create_dynamic_fee_vault<'info>(
    token_mint: &Box<InterfaceAccount<'info, Mint>>,
    params: &InitializeFeeVaultParameters,
    fee_vault: &AccountLoader<'info, DynamicFeeVault>,
    owner: &Pubkey,
    token_vault: &Pubkey,
    base: &Pubkey,
    vault_bump: u8,
    vault_type: u8,
) -> Result<()> {
    require!(is_supported_mint(&token_mint)?, FeeVaultError::InvalidMint);

    // TODO: determine reasonable amount of user for vault at creation. 100 won't fit in a tx
    params.validate(MAX_DYNAMIC_FEE_VAULT_USER)?;

    let mut vault = fee_vault.load_init()?;
    vault.initialize_header(
        owner,
        get_token_program_flags(&token_mint).into(),
        &token_mint.key(),
        token_vault,
        base,
        vault_bump,
        vault_type,
    );

    drop(vault); // drop before re-borrowing the account data

    let fee_vault_info = fee_vault.to_account_info();
    let mut vault = d_load_mut_unchecked::<DynamicFeeVault, UserFee>(&fee_vault_info)?;

    require!(
        vault.dynamic.len() == params.users.len(),
        FeeVaultError::InvalidFeeVaultParameters
    );

    let mut total_share = 0;
    for (i, user) in params.users.iter().enumerate() {
        vault.dynamic[i] = UserFee {
            address: user.address,
            share: user.share,
            ..Default::default()
        };
        total_share = total_share.safe_add(user.share)?;
    }
    vault.fixed.total_share = total_share;

    Ok(())
}

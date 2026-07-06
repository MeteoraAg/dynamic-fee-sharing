use crate::constants::seeds::WHITELISTED_ACTION_PREFIX;
use crate::event::EvtFundByWhitelistedAction;
use crate::state::{DynamicFeeVault, DynamicUserFee, VaultOps, VaultType, WhitelistedAction};
use crate::utils::d_load_mut_checked;
use crate::{error::FeeVaultError, math::SafeMath};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};
use anchor_spl::token_interface::TokenAccount;

#[event_cpi]
#[derive(Accounts)]
#[instruction(discriminator: [u8; 8])]
pub struct FundByWhitelistedActionCtx<'info> {
    #[account(mut)]
    pub dynamic_fee_vault: AccountLoader<'info, DynamicFeeVault>,

    #[account(mut)]
    pub token_0_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub token_1_vault: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// signer, must be a share holder of the vault
    pub signer: Signer<'info>,

    #[account(
        seeds = [
            WHITELISTED_ACTION_PREFIX.as_ref(),
            source_program.key().as_ref(),
            discriminator.as_ref(),
        ],
        bump,
    )]
    pub whitelisted_action: Account<'info, WhitelistedAction>,

    /// CHECK: source origram
    pub source_program: UncheckedAccount<'info>,
}

pub fn validate_token_vaults(
    whitelisted_action: &WhitelistedAction,
    token_0_vault: &Pubkey,
    token_1_vault: Option<&Pubkey>,
    remaining_accounts: &[AccountInfo],
) -> bool {
    let index_0: usize = whitelisted_action.token_0_vault_index.into();
    let Some(account_0) = remaining_accounts.get(index_0) else {
        return false;
    };
    if token_0_vault.ne(account_0.key) {
        return false;
    }

    if whitelisted_action.has_token_1() {
        let Some(token_1_vault) = token_1_vault else {
            return false;
        };
        let index_1: usize = whitelisted_action.token_1_vault_index.into();
        let Some(account_1) = remaining_accounts.get(index_1) else {
            return false;
        };
        if token_1_vault.ne(account_1.key) {
            return false;
        }
    }

    true
}

pub fn handle_fund_by_whitelisted_action(
    ctx: Context<FundByWhitelistedActionCtx>,
    discriminator: [u8; 8],
    payload: Vec<u8>,
) -> Result<()> {
    require!(payload.len() >= 8, FeeVaultError::InvalidAction);
    require!(
        payload[..8].eq(&discriminator),
        FeeVaultError::InvalidAction
    );

    let has_token_1 = ctx.accounts.whitelisted_action.has_token_1();
    let token_0_vault_key = ctx.accounts.token_0_vault.key();
    let token_1_vault_key = ctx.accounts.token_1_vault.as_ref().map(|vault| vault.key());

    let fee_vault_key = ctx.accounts.dynamic_fee_vault.key();
    let fee_vault_info = ctx.accounts.dynamic_fee_vault.to_account_info();

    let vault = d_load_mut_checked::<DynamicFeeVault, DynamicUserFee>(&fee_vault_info)?;
    require!(
        vault.is_share_holder(ctx.accounts.signer.key),
        FeeVaultError::InvalidSigner
    );

    require!(
        vault.fixed.vault_type == u8::from(VaultType::PdaAccount),
        FeeVaultError::InvalidFeeVault
    );

    require!(
        token_0_vault_key.eq(&vault.fixed.token_0_vault),
        FeeVaultError::InvalidFeeVault
    );

    if has_token_1 {
        let token_1_vault_key = token_1_vault_key.ok_or_else(|| FeeVaultError::InvalidFeeVault)?;
        require!(
            token_1_vault_key.eq(&vault.fixed.token_1_vault),
            FeeVaultError::InvalidFeeVault
        );
    }

    let base = vault.fixed.base;
    let token_0_mint = vault.fixed.token_0_mint;
    let token_1_mint = vault.fixed.token_1_mint;
    let vault_bump = vault.fixed.vault_bump;
    drop(vault);

    require!(
        validate_token_vaults(
            &ctx.accounts.whitelisted_action,
            &token_0_vault_key,
            token_1_vault_key.as_ref(),
            ctx.remaining_accounts,
        ),
        FeeVaultError::InvalidAction
    );

    let before_token_0_balance = ctx.accounts.token_0_vault.amount;
    let before_token_1_balance = ctx
        .accounts
        .token_1_vault
        .as_ref()
        .map(|vault| vault.amount);

    let accounts: Vec<AccountMeta> = ctx
        .remaining_accounts
        .iter()
        .map(|acc| AccountMeta {
            pubkey: *acc.key,
            is_signer: acc.key.eq(&fee_vault_key),
            is_writable: acc.is_writable,
        })
        .collect();

    let account_infos: Vec<AccountInfo> = ctx
        .remaining_accounts
        .iter()
        .map(|acc| AccountInfo { ..acc.clone() })
        .collect();

    let signer_seeds = dynamic_fee_vault_seeds!(base, token_0_mint, token_1_mint, vault_bump);

    invoke_signed(
        &Instruction {
            program_id: ctx.accounts.source_program.key(),
            accounts,
            data: payload.clone(),
        },
        &account_infos,
        &[signer_seeds],
    )?;

    ctx.accounts.token_0_vault.reload()?;
    let claimed_amount_0 = ctx
        .accounts
        .token_0_vault
        .amount
        .safe_sub(before_token_0_balance)?;

    let claimed_amount_1 = if has_token_1 {
        let token_1_vault = ctx
            .accounts
            .token_1_vault
            .as_mut()
            .ok_or_else(|| FeeVaultError::InvalidFeeVault)?;
        token_1_vault.reload()?;
        let before_token_1_balance =
            before_token_1_balance.ok_or_else(|| FeeVaultError::InvalidFeeVault)?;
        token_1_vault.amount.safe_sub(before_token_1_balance)?
    } else {
        0
    };

    if claimed_amount_0 == 0 && claimed_amount_1 == 0 {
        return Ok(());
    }

    let mut vault = d_load_mut_checked::<DynamicFeeVault, DynamicUserFee>(&fee_vault_info)?;
    let mut fee_per_share_0 = vault.fixed.fee_per_share_token_0;
    let mut fee_per_share_1 = vault.fixed.fee_per_share_token_1;
    if claimed_amount_0 > 0 {
        fee_per_share_0 = vault.fund_fee(true, claimed_amount_0)?;
    }
    if claimed_amount_1 > 0 {
        fee_per_share_1 = vault.fund_fee(false, claimed_amount_1)?;
    }
    drop(vault);

    emit_cpi!(EvtFundByWhitelistedAction {
        fee_vault: fee_vault_key,
        source_program: ctx.accounts.source_program.key(),
        whitelisted_action: ctx.accounts.whitelisted_action.key(),
        funded_amount_0: claimed_amount_0,
        funded_amount_1: claimed_amount_1,
        fee_per_share_0,
        fee_per_share_1,
        payload,
    });

    Ok(())
}

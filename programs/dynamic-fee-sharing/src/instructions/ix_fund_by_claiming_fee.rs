use crate::constants::WHITELISTED_ACTIONS;
use crate::event::EvtFundFee;
use crate::state::FeeVault;
use crate::{error::FeeVaultError, math::SafeMath};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token_interface::TokenAccount;

#[event_cpi]
#[derive(Accounts)]
pub struct FundByClaimingFeeCtx<'info> {
    #[account(mut, has_one = token_vault)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    #[account(mut)]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// signer
    pub signer: Signer<'info>,

    /// CHECK:: source program
    pub source_program: UncheckedAccount<'info>,
}

pub fn is_support_action(source_program: &Pubkey, discriminator: &[u8]) -> bool {
    for &(program, disc) in WHITELISTED_ACTIONS.iter() {
        if program.eq(source_program) && disc.eq(discriminator) {
            return true;
        }
    }
    false
}

pub fn handle_fund_by_claiming_fee(
    ctx: Context<FundByClaimingFeeCtx>,
    payload: Vec<u8>,
) -> Result<()> {
    let discriminator = &payload[..8]; // first 8 bytes is discriminator
    require!(
        is_support_action(ctx.accounts.source_program.key, discriminator),
        FeeVaultError::InvalidAction
    );

    let fee_vault = ctx.accounts.fee_vault.load()?;

    require!(
        fee_vault.is_share_holder(ctx.accounts.signer.key),
        FeeVaultError::InvalidSigner
    );

    // support fee vault type is pda account
    require!(
        fee_vault.fee_vault_type == 1,
        FeeVaultError::InvalidFeeVault
    );

    let before_token_vault_balance = ctx.accounts.token_vault.amount;

    {
        let accounts: Vec<AccountMeta> = ctx
            .remaining_accounts
            .iter()
            .map(|acc| {
                let is_signer = acc.key == &ctx.accounts.fee_vault.key();
                AccountMeta {
                    pubkey: *acc.key,
                    is_signer: is_signer,
                    is_writable: acc.is_writable,
                }
            })
            .collect();

        let account_infos: Vec<AccountInfo> = ctx
            .remaining_accounts
            .iter()
            .map(|acc| AccountInfo { ..acc.clone() })
            .collect();
        // invoke instruction to amm
        let base = fee_vault.base;
        let token_mint = fee_vault.token_mint;
        let fee_vault_bump = fee_vault.fee_vault_bump;
        let signer_seeds = fee_vault_seeds!(base, token_mint, fee_vault_bump);
        drop(fee_vault);

        invoke_signed(
            &Instruction {
                program_id: ctx.accounts.source_program.key(),
                accounts,
                data: payload.clone(),
            },
            &account_infos,
            &[signer_seeds],
        )?;
    }

    ctx.accounts.token_vault.reload()?;

    let after_token_vault_balance = ctx.accounts.token_vault.amount;

    let claimed_amount = after_token_vault_balance.safe_sub(before_token_vault_balance)?;

    if claimed_amount > 0 {
        let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
        fee_vault.fund_fee(claimed_amount)?;

        emit_cpi!(EvtFundFee {
            source_program: ctx.accounts.source_program.key(),
            fee_vault: ctx.accounts.fee_vault.key(),
            payload,
            funded_amount: claimed_amount,
            fee_per_share: fee_vault.fee_per_share,
        });
    }
    Ok(())
}

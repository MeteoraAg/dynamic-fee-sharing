use crate::constants::seeds::DAMM_V2_POSITION_NFT_ACCOUNT_PREFIX;
use crate::error::FeeVaultError;
use crate::event::EvtReclaimDammV2Position;
use crate::math::SafeCast;
use crate::state::{FeeVault, FeeVaultType};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::{spl_token_2022, Token2022};
use anchor_spl::token_interface::TokenAccount;

#[event_cpi]
#[derive(Accounts)]
pub struct ReclaimDammV2PositionCtx<'info> {
    pub fee_vault: AccountLoader<'info, FeeVault>,

    #[account(mut)]
    pub position_nft_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: new owner of the position nft account
    pub new_owner: UncheckedAccount<'info>,

    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
}

pub fn handle_reclaim_damm_v2_position(ctx: Context<ReclaimDammV2PositionCtx>) -> Result<()> {
    require!(
        ctx.accounts
            .position_nft_account
            .owner
            .eq(&ctx.accounts.fee_vault.key()),
        FeeVaultError::Unauthorized
    );

    let (expected_pda, _) = Pubkey::find_program_address(
        &[
            DAMM_V2_POSITION_NFT_ACCOUNT_PREFIX,
            ctx.accounts.position_nft_account.mint.as_ref(),
        ],
        &damm_v2::ID,
    );

    require!(
        ctx.accounts.position_nft_account.key() == expected_pda,
        FeeVaultError::InvalidAction
    );

    let fee_vault = ctx.accounts.fee_vault.load()?;

    require!(
        fee_vault.fee_vault_type.safe_cast()? == FeeVaultType::PdaAccount,
        FeeVaultError::InvalidFeeVault
    );

    let base = fee_vault.base;
    let token_mint = fee_vault.token_mint;
    let fee_vault_bump = fee_vault.fee_vault_bump;
    let signer_seeds = fee_vault_seeds!(base, token_mint, fee_vault_bump);
    drop(fee_vault);

    let instruction = spl_token_2022::instruction::set_authority(
        ctx.accounts.token_program.key,
        &ctx.accounts.position_nft_account.key(),
        Some(&ctx.accounts.new_owner.key()),
        spl_token_2022::instruction::AuthorityType::AccountOwner,
        &ctx.accounts.fee_vault.key(),
        &[],
    )?;

    invoke_signed(
        &instruction,
        &[
            ctx.accounts.position_nft_account.to_account_info(),
            ctx.accounts.fee_vault.to_account_info(),
        ],
        &[signer_seeds],
    )?;

    emit_cpi!(EvtReclaimDammV2Position {
        fee_vault: ctx.accounts.fee_vault.key(),
        position_nft_account: ctx.accounts.position_nft_account.key(),
        new_owner: ctx.accounts.new_owner.key(),
    });

    Ok(())
}

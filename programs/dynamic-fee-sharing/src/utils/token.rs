use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use anchor_spl::{
    token::Token,
    token_2022::spl_token_2022::{
        self,
        extension::{
            self, transfer_fee::TransferFee, BaseStateWithExtensions, ExtensionType,
            StateWithExtensions,
        },
    },
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::error::FeeVaultError;
#[derive(
    AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq, IntoPrimitive, TryFromPrimitive,
)]
#[repr(u8)]
pub enum TokenProgramFlags {
    TokenProgram,
    TokenProgram2022,
}

pub fn get_token_program_flags<'a, 'info>(
    token_mint: &'a InterfaceAccount<'info, Mint>,
) -> TokenProgramFlags {
    let token_mint_ai = token_mint.to_account_info();

    if token_mint_ai.owner.eq(&anchor_spl::token::ID) {
        TokenProgramFlags::TokenProgram
    } else {
        TokenProgramFlags::TokenProgram2022
    }
}

pub fn is_supported_mint(mint_account: &InterfaceAccount<Mint>) -> Result<bool> {
    let mint_info = mint_account.to_account_info();
    if *mint_info.owner == Token::id() {
        return Ok(true);
    }

    let mint_data = mint_info.try_borrow_data()?;
    let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;
    let extensions = mint.get_extension_types()?;
    for e in extensions {
        if e != ExtensionType::TransferFeeConfig
            && e != ExtensionType::MetadataPointer
            && e != ExtensionType::TokenMetadata
        {
            return Ok(false);
        }
    }
    Ok(true)
}

#[derive(Debug)]
pub struct TransferFeeExcludedAmount {
    pub amount: u64,
    pub transfer_fee: u64,
}

pub fn calculate_transfer_fee_excluded_amount<'info>(
    token_mint: &InterfaceAccount<'info, Mint>,
    transfer_fee_included_amount: u64,
) -> Result<TransferFeeExcludedAmount> {
    if let Some(epoch_transfer_fee) = get_epoch_transfer_fee(token_mint)? {
        let transfer_fee = epoch_transfer_fee
            .calculate_fee(transfer_fee_included_amount)
            .ok_or_else(|| FeeVaultError::MathOverflow)?;
        let transfer_fee_excluded_amount = transfer_fee_included_amount
            .checked_sub(transfer_fee)
            .ok_or_else(|| FeeVaultError::MathOverflow)?;
        return Ok(TransferFeeExcludedAmount {
            amount: transfer_fee_excluded_amount,
            transfer_fee,
        });
    }

    Ok(TransferFeeExcludedAmount {
        amount: transfer_fee_included_amount,
        transfer_fee: 0,
    })
}

pub fn get_epoch_transfer_fee<'info>(
    token_mint: &InterfaceAccount<'info, Mint>,
) -> Result<Option<TransferFee>> {
    let token_mint_info = token_mint.to_account_info();
    if *token_mint_info.owner == Token::id() {
        return Ok(None);
    }

    let token_mint_data = token_mint_info.try_borrow_data()?;
    let token_mint_unpacked =
        StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint_data)?;
    if let Ok(transfer_fee_config) =
        token_mint_unpacked.get_extension::<extension::transfer_fee::TransferFeeConfig>()
    {
        let epoch = Clock::get()?.epoch;
        return Ok(Some(transfer_fee_config.get_epoch_fee(epoch).clone()));
    }

    Ok(None)
}

pub fn transfer_from_user<'a, 'c: 'info, 'info>(
    authority: &'a Signer<'info>,
    token_mint: &'a InterfaceAccount<'info, Mint>,
    token_owner_account: &'a InterfaceAccount<'info, TokenAccount>,
    destination_token_account: &'a InterfaceAccount<'info, TokenAccount>,
    token_program: &'a Interface<'info, TokenInterface>,
    amount: u64,
) -> Result<()> {
    let destination_account = destination_token_account.to_account_info();

    let instruction = spl_token_2022::instruction::transfer_checked(
        token_program.key,
        &token_owner_account.key(),
        &token_mint.key(),
        destination_account.key,
        authority.key,
        &[],
        amount,
        token_mint.decimals,
    )?;

    let account_infos = vec![
        token_owner_account.to_account_info(),
        token_mint.to_account_info(),
        destination_account.to_account_info(),
        authority.to_account_info(),
    ];

    invoke_signed(&instruction, &account_infos, &[])?;

    Ok(())
}

pub fn transfer_from_fee_vault<'c: 'info, 'info>(
    pool_authority: AccountInfo<'info>,
    token_mint: &InterfaceAccount<'info, Mint>,
    token_vault: &InterfaceAccount<'info, TokenAccount>,
    token_owner_account: &InterfaceAccount<'info, TokenAccount>,
    token_program: &Interface<'info, TokenInterface>,
    amount: u64,
) -> Result<()> {
    let signer_seeds = fee_vault_authority_seeds!();

    let instruction = spl_token_2022::instruction::transfer_checked(
        token_program.key,
        &token_vault.key(),
        &token_mint.key(),
        &token_owner_account.key(),
        &pool_authority.key(),
        &[],
        amount,
        token_mint.decimals,
    )?;

    let account_infos = vec![
        token_vault.to_account_info(),
        token_mint.to_account_info(),
        token_owner_account.to_account_info(),
        pool_authority.to_account_info(),
    ];

    invoke_signed(&instruction, &account_infos, &[&signer_seeds[..]])?;

    Ok(())
}

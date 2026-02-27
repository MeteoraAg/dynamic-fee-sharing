use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke_signed, system_instruction},
};
use anchor_spl::{
    token::{Token, TokenAccount},
    token_2022::{
        spl_token_2022::{
            self,
            extension::{
                self, transfer_fee::TransferFee, BaseStateWithExtensions, ExtensionType,
                StateWithExtensions,
            },
        },
        Token2022,
    },
    token_interface::{Mint, TokenAccount},
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

pub fn transfer_from_user<'a, 'info>(
    authority: &'a Signer<'info>,
    token_mint: &'a InterfaceAccount<'info, Mint>,
    token_owner_account: &'a InterfaceAccount<'info, TokenAccount>,
    destination_token_account: &'a InterfaceAccount<'info, TokenAccount>,
    token_program: &'a AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let instruction = spl_token_2022::instruction::transfer_checked(
        token_program.key,
        token_owner_account.key,
        &token_mint.key(),
        destination_token_account.key,
        authority.key,
        &[],
        amount,
        token_mint.decimals,
    )?;

    let account_infos = vec![
        token_owner_account,
        token_mint.to_account_info(),
        destination_token_account,
        authority,
    ];

    invoke_signed(&instruction, &account_infos, &[])?;

    Ok(())
}

pub fn transfer_from_fee_vault<'info>(
    pool_authority: AccountInfo<'info>,
    token_mint: &InterfaceAccount<'info, Mint>,
    token_vault: &InterfaceAccount<'info, TokenAccount>,
    token_owner_account: &InterfaceAccount<'info, TokenAccount>,
    token_program: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let signer_seeds = fee_vault_authority_seeds!();

    let instruction = spl_token_2022::instruction::transfer_checked(
        token_program.key,
        token_vault.key,
        &token_mint.key(),
        token_owner_account.key,
        pool_authority.key,
        &[],
        amount,
        token_mint.decimals,
    )?;

    let account_infos = vec![
        token_vault,
        token_mint.to_account_info(),
        token_owner_account,
        pool_authority,
    ];

    invoke_signed(&instruction, &account_infos, &[&signer_seeds[..]])?;

    Ok(())
}

pub fn create_pda_token_account<'info>(
    payer: AccountInfo<'info>,
    new_account: AccountInfo<'info>,
    mint: &InterfaceAccount<'info, Mint>,
    authority: &Pubkey,
    token_program: &Interface<'info, TokenInterface>,
    system_program: AccountInfo<'info>,
    signer_seeds: &[&[u8]],
) -> Result<()> {
    let space = get_token_account_space(mint)?;
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            new_account.key,
            lamports,
            space as u64,
            token_program.key,
        ),
        &[payer, new_account.clone(), system_program],
        &[signer_seeds],
    )?;

    invoke_signed(
        &spl_token_2022::instruction::initialize_account3(
            token_program.key,
            new_account.key,
            &mint.key(),
            authority,
        )?,
        &[new_account, mint.to_account_info()],
        &[],
    )?;

    Ok(())
}

// refrence https://github.com/solana-foundation/anchor/blob/1ebbe58158d089a2a40b5e35ebead5a10db9090d/lang/syn/src/codegen/accounts/constraints.rs#L1599
fn get_token_account_space(mint: &InterfaceAccount<Mint>) -> Result<usize> {
    let mint_info = mint.to_account_info();
    if *mint_info.owner == Token2022::id() {
        let mint_data = mint_info.try_borrow_data()?;
        let unpacked = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;
        let mint_extensions = unpacked.get_extension_types()?;
        let required_extensions =
            ExtensionType::get_required_init_account_extensions(&mint_extensions);
        ExtensionType::try_calculate_account_len::<spl_token_2022::state::Account>(
            &required_extensions,
        )
        .map_err(|_| error!(FeeVaultError::MathOverflow))
    } else {
        Ok(TokenAccount::LEN)
    }
}

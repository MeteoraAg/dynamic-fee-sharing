use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::solana_program::system_instruction;

/// refer the code from https://github.com/solana-program/associated-token-account/blob/28cbfb701bb791ab74b912e5e489731e7c79e164/program/src/tools/account.rs#L19
pub fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    rent: &Rent,
    space: usize,
    owner: &Pubkey,
    system_program: &AccountInfo<'a>,
    new_pda_account: &AccountInfo<'a>,
    new_pda_signer_seeds: &[&[u8]],
) -> Result<()> {
    if new_pda_account.lamports() > 0 {
        let required_lamports = rent
            .minimum_balance(space)
            .max(1)
            .saturating_sub(new_pda_account.lamports());

        if required_lamports > 0 {
            invoke(
                &system_instruction::transfer(payer.key, new_pda_account.key, required_lamports),
                &[
                    payer.clone(),
                    new_pda_account.clone(),
                    system_program.clone(),
                ],
            )?;
        }

        invoke_signed(
            &system_instruction::allocate(new_pda_account.key, space as u64),
            &[new_pda_account.clone(), system_program.clone()],
            &[new_pda_signer_seeds],
        )?;

        invoke_signed(
            &system_instruction::assign(new_pda_account.key, owner),
            &[new_pda_account.clone(), system_program.clone()],
            &[new_pda_signer_seeds],
        )?;
    } else {
        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                new_pda_account.key,
                rent.minimum_balance(space).max(1),
                space as u64,
                owner,
            ),
            &[
                payer.clone(),
                new_pda_account.clone(),
                system_program.clone(),
            ],
            &[new_pda_signer_seeds],
        )?;
    }

    Ok(())
}

/// Creates a PDA account and writes the Anchor discriminator.
pub fn create_pda_account_with_anchor_discriminator<'a, T: Discriminator + Space>(
    payer: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    new_pda_account: &AccountInfo<'a>,
    new_pda_signer_seeds: &[&[u8]],
) -> Result<()> {
    let space = T::DISCRIMINATOR.len() + T::INIT_SPACE;
    let rent = Rent::get()?;

    create_pda_account(
        payer,
        &rent,
        space,
        &crate::ID,
        system_program,
        new_pda_account,
        new_pda_signer_seeds,
    )?;

    let mut data = new_pda_account.try_borrow_mut_data()?;
    data[..T::DISCRIMINATOR.len()].copy_from_slice(T::DISCRIMINATOR);

    Ok(())
}

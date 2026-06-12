use anchor_lang::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_lang::system_program::System;
use bytemuck::Pod;
use std::cell::RefMut;

fn validate_discriminator_and_owner<'a, F: Discriminator + Owner>(
    acc_info: &'a AccountInfo,
) -> Result<()> {
    if acc_info.owner != &F::owner() {
        return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
            .with_pubkeys((*acc_info.owner, F::owner())));
    }

    let data = acc_info.try_borrow_data()?;
    let disc = F::DISCRIMINATOR;
    if data.len() < disc.len() {
        return Err(ErrorCode::AccountDiscriminatorNotFound.into());
    }

    let given_disc = &data[..disc.len()];
    if given_disc != disc {
        return Err(ErrorCode::AccountDiscriminatorMismatch.into());
    }
    Ok(())
}

pub fn load_mut_checked<'a, O: Discriminator + Owner, T: Pod + Space>(
    acc_info: &'a AccountInfo,
) -> Result<RefMut<'a, T>> {
    validate_discriminator_and_owner::<O>(acc_info)?;
    load_mut_unchecked::<O, T>(acc_info)
}

pub fn load_mut_unchecked<'a, O: Discriminator, T: Pod + Space>(
    acc_info: &'a AccountInfo,
) -> Result<RefMut<'a, T>> {
    let data = acc_info.try_borrow_mut_data()?;

    Ok(RefMut::map(data, |data| {
        // just panic if it is wrong
        bytemuck::from_bytes_mut(
            &mut data[O::DISCRIMINATOR.len()..(T::INIT_SPACE + O::DISCRIMINATOR.len())],
        )
    }))
}

pub fn is_account_initialized<'a, T: Pod + Owner + Discriminator>(
    acc_info: &AccountInfo<'a>,
) -> Result<bool> {
    let data = acc_info.try_borrow_data()?;
    if acc_info.owner.eq(&System::id()) && data.len() == 0 {
        Ok(false)
    } else {
        validate_discriminator_and_owner::<T>(acc_info)?;
        Ok(true)
    }
}

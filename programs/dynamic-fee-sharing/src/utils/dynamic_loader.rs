use anchor_lang::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_lang::{Discriminator, Space};
use bytemuck::Pod;
use std::cell::{Ref, RefMut};

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

#[derive(Debug)]
pub struct DynamicAccountMut<'a, F, D> {
    pub fixed: RefMut<'a, F>,
    pub dynamic: RefMut<'a, [D]>,
}
impl<'a, F, D> DynamicAccountMut<'a, F, D> {
    pub fn length(&self) -> usize {
        self.dynamic.len()
    }
}

pub fn d_load_mut_checked<'a, F: Pod + Space + Discriminator + Owner, D: Pod + Space>(
    acc_info: &'a AccountInfo,
) -> Result<DynamicAccountMut<'a, F, D>> {
    validate_discriminator_and_owner::<F>(acc_info)?;
    d_load_mut_unchecked::<F, D>(acc_info)
}

pub fn d_load_mut_unchecked<'a, F: Pod + Space, D: Pod + Space>(
    acc_info: &'a AccountInfo,
) -> Result<DynamicAccountMut<'a, F, D>> {
    let data = acc_info.try_borrow_mut_data()?;

    let (fixed, dynamic) = RefMut::map_split(data, |data| {
        let (fixed_bytes, dynamic_bytes) = data.split_at_mut(8 + F::INIT_SPACE);
        let fixed = bytemuck::from_bytes_mut::<F>(&mut fixed_bytes[8..]);
        let dynamic = bytemuck::cast_slice_mut::<u8, D>(dynamic_bytes);

        (fixed, dynamic)
    });

    Ok(DynamicAccountMut { fixed, dynamic })
}

#[derive(Debug)]
pub struct DynamicAccount<'a, F, D> {
    pub fixed: Ref<'a, F>,
    pub dynamic: Ref<'a, [D]>,
}
impl<'a, F, D> DynamicAccount<'a, F, D> {
    pub fn length(&self) -> usize {
        self.dynamic.len()
    }
}

pub fn d_load_checked<'a, F: Pod + Space + Discriminator + Owner, D: Pod + Space>(
    acc_info: &'a AccountInfo,
) -> Result<DynamicAccount<'a, F, D>> {
    validate_discriminator_and_owner::<F>(acc_info)?;
    d_load_unchecked::<F, D>(acc_info)
}

pub fn d_load_unchecked<'a, F: Pod + Space, D: Pod + Space>(
    acc_info: &'a AccountInfo,
) -> Result<DynamicAccount<'a, F, D>> {
    let data = acc_info.try_borrow_data()?;

    let (fixed, dynamic) = Ref::map_split(data, |data| {
        let (fixed_bytes, dynamic_bytes) = data.split_at(8 + F::INIT_SPACE);
        let fixed = bytemuck::from_bytes::<F>(&fixed_bytes[8..]);
        let dynamic = bytemuck::cast_slice::<u8, D>(dynamic_bytes);
        (fixed, dynamic)
    });

    Ok(DynamicAccount { fixed, dynamic })
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
        bytemuck::from_bytes_mut(
            &mut data[O::DISCRIMINATOR.len()..(T::INIT_SPACE + O::DISCRIMINATOR.len())],
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{DynamicFeeVault, UserFee};

    // Guards that the dynamic fee vault types satisfy the loader bounds, since
    // generic bounds are only checked where the loader is instantiated.
    fn _assert_loadable() {
        fn check<F: Pod + Space + Discriminator + Owner, D: Pod + Space>() {}
        check::<DynamicFeeVault, UserFee>();
    }
}

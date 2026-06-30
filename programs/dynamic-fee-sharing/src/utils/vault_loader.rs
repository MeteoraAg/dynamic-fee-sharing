use crate::error::FeeVaultError;
use crate::state::{DynamicFeeVault, FeeVault, UserFee, VaultHeader, VaultOps};
use crate::utils::{d_load_mut_checked, load_mut_checked, DynamicAccountMut};
use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use std::cell::RefMut;

pub enum Vault<'a> {
    Fixed(RefMut<'a, FeeVault>),
    Dynamic(DynamicAccountMut<'a, DynamicFeeVault, UserFee>),
}

pub fn load_vault_mut<'a>(acc: &'a AccountInfo) -> Result<Vault<'a>> {
    let disc: [u8; 8] = {
        let data = acc.try_borrow_data()?;
        require!(data.len() >= 8, FeeVaultError::InvalidFeeVault);
        data[..8]
            .try_into()
            .map_err(|_| FeeVaultError::InvalidFeeVault)?
    };

    if disc == FeeVault::DISCRIMINATOR {
        Ok(Vault::Fixed(load_mut_checked::<FeeVault, FeeVault>(acc)?))
    } else if disc == DynamicFeeVault::DISCRIMINATOR {
        Ok(Vault::Dynamic(d_load_mut_checked::<
            DynamicFeeVault,
            UserFee,
        >(acc)?))
    } else {
        Err(FeeVaultError::InvalidFeeVault.into())
    }
}

impl VaultOps for Vault<'_> {
    fn get_header_and_users(&self) -> (&VaultHeader, &[UserFee]) {
        match self {
            Vault::Fixed(v) => {
                let fv: &FeeVault = v;
                (&fv.fixed, &fv.users[..])
            }
            Vault::Dynamic(v) => (&v.fixed.fixed, &v.dynamic[..]),
        }
    }

    fn get_header_and_users_mut(&mut self) -> (&mut VaultHeader, &mut [UserFee]) {
        match self {
            Vault::Fixed(v) => {
                let fv: &mut FeeVault = v;
                (&mut fv.fixed, &mut fv.users[..])
            }
            Vault::Dynamic(v) => (&mut v.fixed.fixed, &mut v.dynamic[..]),
        }
    }
}

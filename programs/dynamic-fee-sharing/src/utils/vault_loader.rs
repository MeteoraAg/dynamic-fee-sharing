use crate::error::FeeVaultError;
use crate::state::{DynamicFeeVault, DynamicUserFee, FeeVault, VaultOps};
use crate::utils::{d_load_mut_checked, load_mut_checked};
use anchor_lang::prelude::*;
use anchor_lang::Discriminator;

pub fn load_vault_mut<'a>(acc: &'a AccountInfo) -> Result<Box<dyn VaultOps + 'a>> {
    let disc: [u8; 8] = {
        let data = acc.try_borrow_data()?;
        require!(data.len() >= 8, FeeVaultError::InvalidFeeVault);
        data[..8]
            .try_into()
            .map_err(|_| FeeVaultError::InvalidFeeVault)?
    };

    if disc == FeeVault::DISCRIMINATOR {
        Ok(Box::new(load_mut_checked::<FeeVault, FeeVault>(acc)?))
    } else if disc == DynamicFeeVault::DISCRIMINATOR {
        Ok(Box::new(d_load_mut_checked::<
            DynamicFeeVault,
            DynamicUserFee,
        >(acc)?))
    } else {
        Err(FeeVaultError::InvalidFeeVault.into())
    }
}

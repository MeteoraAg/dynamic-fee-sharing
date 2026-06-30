use crate::constants::MIN_USER;
use crate::error::FeeVaultError;
use anchor_lang::prelude::*;

// Shared init parameters for both `initialize_fee_vault` (fixed `FeeVault`) and
// `initialize_dynamic_fee_vault` (`DynamicFeeVault`). `validate` takes the per-type
// max user bound so each instruction enforces its own ceiling.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct InitializeFeeVaultParameters {
    pub padding: [u64; 8], // for future use
    pub users: Vec<UserShare>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
pub struct UserShare {
    pub address: Pubkey,
    pub share: u32,
}

impl InitializeFeeVaultParameters {
    pub fn validate(&self, max_user: usize) -> Result<()> {
        let number_of_user = self.users.len();
        require!(
            number_of_user >= MIN_USER && number_of_user <= max_user,
            FeeVaultError::ExceededUser
        );
        for i in 0..number_of_user {
            require!(
                self.users[i].share > 0,
                FeeVaultError::InvalidFeeVaultParameters
            );
            require!(
                self.users[i].address.ne(&Pubkey::default()),
                FeeVaultError::InvalidUserAddress
            );
        }
        // that is fine to leave user addresses are duplicated?
        Ok(())
    }
}

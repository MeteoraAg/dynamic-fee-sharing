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
    pub fn validate(&self, max_user: usize, allow_zero_share: bool) -> Result<()> {
        let number_of_user = self.users.len();
        require!(
            number_of_user >= MIN_USER && number_of_user <= max_user,
            FeeVaultError::ExceededUser
        );
        for (i, user) in self.users.iter().enumerate() {
            if !allow_zero_share {
                require!(user.share > 0, FeeVaultError::InvalidFeeVaultParameters);
            }
            require!(
                user.address.ne(&Pubkey::default()),
                FeeVaultError::InvalidUserAddress
            );

            let duplicated = self.users[i + 1..]
                .iter()
                .any(|u| u.address.eq(&user.address));

            require!(!duplicated, FeeVaultError::DuplicatedUserAddress);
        }
        Ok(())
    }
}

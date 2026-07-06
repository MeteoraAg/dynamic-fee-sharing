use crate::error::FeeVaultError;
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct CreateWhitelistedActionParameters {
    pub source_program: Pubkey,
    pub discriminator: [u8; 8],
    pub token_0_vault_index: u8,
    /// u8::MAX = action claims one token only
    pub token_1_vault_index: u8,
    pub padding: [u64; 4], // for future use
}

impl CreateWhitelistedActionParameters {
    pub fn validate(&self) -> Result<()> {
        require!(
            self.source_program.ne(&Pubkey::default()),
            FeeVaultError::InvalidWhitelistedActionParameters
        );
        // block whitelisting our own program (reentrancy via self-CPI)
        require!(
            self.source_program.ne(&crate::ID),
            FeeVaultError::InvalidWhitelistedActionParameters
        );
        require!(
            self.token_0_vault_index != u8::MAX,
            FeeVaultError::InvalidWhitelistedActionParameters
        );
        if self.token_1_vault_index != u8::MAX {
            require!(
                self.token_1_vault_index != self.token_0_vault_index,
                FeeVaultError::InvalidWhitelistedActionParameters
            );
        }
        Ok(())
    }
}

use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;

use crate::params::CreateWhitelistedActionParameters;

/// whitelisted external fee-claim action
/// one PDA per source_program, discriminator
#[account]
#[derive(InitSpace, Debug)]
pub struct WhitelistedAction {
    pub source_program: Pubkey,
    pub discriminator: [u8; 8],
    pub token_0_vault_index: u8,
    pub token_1_vault_index: u8,
    pub padding: [u8; 64], // for future use
}
const_assert_eq!(WhitelistedAction::INIT_SPACE, 106);

impl WhitelistedAction {
    pub fn initialize(&mut self, params: &CreateWhitelistedActionParameters) {
        self.source_program = params.source_program;
        self.discriminator = params.discriminator;
        self.token_0_vault_index = params.token_0_vault_index;
        self.token_1_vault_index = params.token_1_vault_index;
    }

    pub fn has_token_1(&self) -> bool {
        self.token_1_vault_index != u8::MAX
    }
}

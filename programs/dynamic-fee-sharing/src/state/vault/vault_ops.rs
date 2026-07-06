use anchor_lang::prelude::*;

// Dispatcher shape shared by FeeVault and DynamicFeeVault. Implemented by the loaded
// (borrowed) form of each vault; `load_vault_mut` picks the impl by discriminator, so
// instruction handlers stay vault-kind agnostic.
pub trait VaultOps {
    /// returns whether the matching token slot is token 0 (always true for a fixed vault)
    fn validate_token_accounts(&self, token_vault: &Pubkey, token_mint: &Pubkey) -> Result<bool>;

    /// returns updated fee_per_share for the token slot
    fn fund_fee(&mut self, is_token_0: bool, amount: u64) -> Result<u128>;

    fn validate_and_claim_fee(
        &mut self,
        index: usize,
        is_token_0: bool,
        signer: &Pubkey,
    ) -> Result<u64>;
}

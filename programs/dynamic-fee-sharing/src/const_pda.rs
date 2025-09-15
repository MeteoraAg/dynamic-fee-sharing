use anchor_lang::solana_program::pubkey::Pubkey;
use const_crypto::ed25519;

pub mod fee_vault_authority {
    use super::*;

    const FEE_VAULT_AUTHORITY_AND_BUMP: ([u8; 32], u8) = ed25519::derive_program_address(
        &[crate::constants::seeds::FEE_VAULT_AUTHORITY_PREFIX],
        &crate::ID_CONST.to_bytes(),
    );

    pub const ID: Pubkey = Pubkey::new_from_array(FEE_VAULT_AUTHORITY_AND_BUMP.0);
    pub const BUMP: u8 = FEE_VAULT_AUTHORITY_AND_BUMP.1;
}

// Potential optimization on event authority too since anchor internally do Pubkey::find_program_address during runtime.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_fee_vault_authority() {
        let (derived_pool_authority, derived_bump) = Pubkey::find_program_address(
            &[crate::constants::seeds::FEE_VAULT_AUTHORITY_PREFIX],
            &crate::ID,
        );
        assert_eq!(fee_vault_authority::ID, derived_pool_authority);
        assert_eq!(fee_vault_authority::BUMP, derived_bump);
    }
}

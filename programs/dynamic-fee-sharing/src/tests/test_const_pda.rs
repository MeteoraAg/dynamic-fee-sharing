use crate::const_pda::fee_vault_authority;
use anchor_lang::solana_program::pubkey::Pubkey;

#[test]
fn test_const_fee_vault_authority() {
    let (derived_pool_authority, derived_bump) = Pubkey::find_program_address(
        &[crate::constants::seeds::FEE_VAULT_AUTHORITY_PREFIX],
        &crate::ID,
    );
    assert_eq!(fee_vault_authority::ID, derived_pool_authority);
    assert_eq!(fee_vault_authority::BUMP, derived_bump);
}

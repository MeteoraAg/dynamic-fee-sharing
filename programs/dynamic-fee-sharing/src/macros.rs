macro_rules! fee_vault_authority_seeds {
    () => {
        &[
            crate::constants::seeds::FEE_VAULT_AUTHORITY_PREFIX,
            &[crate::const_pda::fee_vault_authority::BUMP],
        ]
    };
}

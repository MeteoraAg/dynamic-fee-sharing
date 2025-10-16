macro_rules! fee_vault_authority_seeds {
    () => {
        &[
            crate::constants::seeds::FEE_VAULT_AUTHORITY_PREFIX,
            &[crate::const_pda::fee_vault_authority::BUMP],
        ]
    };
}

macro_rules! fee_vault_seeds {
    ($base:expr, $token_mint:expr, $bump:expr) => {
        &[
            crate::constants::seeds::FEE_VAULT_PREFIX,
            $base.as_ref(),
            $token_mint.as_ref(),
            &[$bump],
        ]
    };
}

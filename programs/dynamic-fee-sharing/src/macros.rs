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

macro_rules! dynamic_fee_vault_seeds {
    ($base:expr, $token_0_mint:expr, $token_1_mint:expr, $bump:expr) => {
        &[
            crate::constants::seeds::DYNAMIC_FEE_VAULT_PREFIX,
            $base.as_ref(),
            $token_0_mint.as_ref(),
            $token_1_mint.as_ref(),
            &[$bump],
        ]
    };
}

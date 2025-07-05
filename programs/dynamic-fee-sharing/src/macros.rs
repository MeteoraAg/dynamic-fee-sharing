//! Macro functions
macro_rules! fee_vault_authority_seeds {
    ($bump:expr) => {
        &[b"fee_vault_authority".as_ref(), &[$bump]]
    };
}

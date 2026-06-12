use crate::{error::FeeVaultError, state::FeeVault};
use anchor_lang::prelude::*;

pub fn verify_is_mutable_and_operator(
    fee_vault: &AccountLoader<FeeVault>,
    signer: &Pubkey,
) -> Result<()> {
    let fee_vault = fee_vault.load()?;

    require!(
        fee_vault.mutable_flag == 1,
        FeeVaultError::FeeVaultNotMutable
    );
    require!(
        fee_vault.operator.eq(signer),
        FeeVaultError::InvalidPermission,
    );
    Ok(())
}

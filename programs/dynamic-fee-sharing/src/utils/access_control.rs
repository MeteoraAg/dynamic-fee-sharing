use crate::{error::FeeVaultError, state::FeeVault};
use anchor_lang::prelude::*;

pub(crate) fn verify_is_mutable_and_admin(fee_vault: &FeeVault, signer: &Signer) -> Result<()> {
    require!(fee_vault.mutable_flag == 1, FeeVaultError::InvalidAction);
    require!(
        fee_vault.owner.eq(&signer.key) || fee_vault.operator.eq(&signer.key),
        FeeVaultError::InvalidPermission,
    );
    Ok(())
}

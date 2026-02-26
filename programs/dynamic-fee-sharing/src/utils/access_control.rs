use crate::{error::FeeVaultError, state::FeeVault};
use anchor_lang::prelude::*;

pub fn verify_is_mutable_and_admin<'info>(
    fee_vault: &AccountLoader<'info, FeeVault>,
    signer: &Pubkey,
) -> Result<()> {
    let fee_vault = fee_vault.load()?;

    require!(fee_vault.mutable_flag == 1, FeeVaultError::InvalidAction);
    require!(
        fee_vault.owner.eq(signer) || fee_vault.operator.eq(signer),
        FeeVaultError::InvalidPermission,
    );
    Ok(())
}

use crate::error::FeeVaultError;
use crate::state::operator::{Operator, OperatorPermission};
use crate::state::FeeVault;
use anchor_lang::prelude::*;

pub fn is_valid_operator_role<'info>(
    fee_vault_loader: &AccountLoader<'info, FeeVault>,
    operator_loader: &AccountLoader<'info, Operator>,
    signer: &Pubkey,
    permission: OperatorPermission,
) -> Result<()> {
    let fee_vault = fee_vault_loader.load()?;
    let operator = operator_loader.load()?;

    if fee_vault.operator_address.eq(&operator_loader.key())
        && operator.whitelisted_address.eq(signer)
        && operator.is_permission_allow(permission)
    {
        Ok(())
    } else {
        err!(FeeVaultError::InvalidPermission)
    }
}

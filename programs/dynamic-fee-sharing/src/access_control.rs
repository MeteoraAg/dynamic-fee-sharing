use anchor_lang::prelude::*;

use crate::{error::FeeVaultError, state::FeeVault};

pub fn is_fee_vault_owner<'info>(
    fee_vault: &AccountLoader<'info, FeeVault>,
    signer: &Pubkey,
) -> Result<()> {
    let fee_vault = fee_vault.load()?;
    require!(fee_vault.owner.eq(signer), FeeVaultError::Unauthorized);
    Ok(())
}

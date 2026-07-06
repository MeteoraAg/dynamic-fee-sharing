use crate::{error::FeeVaultError, instructions::assert_eq_admin};
use anchor_lang::prelude::*;

// check whether the signer is in admin list
pub fn is_admin(signer: &Pubkey) -> Result<()> {
    require!(assert_eq_admin(signer.key()), FeeVaultError::InvalidAdmin);
    Ok(())
}

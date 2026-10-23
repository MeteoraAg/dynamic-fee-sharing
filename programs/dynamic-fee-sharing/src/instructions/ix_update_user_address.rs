use anchor_lang::prelude::*;

use crate::event::EvtUpdateUserAddress;
use crate::state::FeeVault;

#[event_cpi]
#[derive(Accounts)]
pub struct UpdateUserAddressCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

    /// the current address of the user at `index`
    pub user: Signer<'info>,
}

// Lets a user move its share to another address, e.g. to rotate to a new wallet or a multisig.
// Only the current address can sign, so this is not a recovery path once its key is compromised:
// whoever holds the key can update the address first.
pub fn handle_update_user_address(
    ctx: Context<UpdateUserAddressCtx>,
    index: u8,
    new_address: Pubkey,
) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
    let old_address = fee_vault.update_user_address(index, ctx.accounts.user.key, &new_address)?;

    emit_cpi!(EvtUpdateUserAddress {
        fee_vault: ctx.accounts.fee_vault.key(),
        index,
        old_address,
        new_address,
    });

    Ok(())
}

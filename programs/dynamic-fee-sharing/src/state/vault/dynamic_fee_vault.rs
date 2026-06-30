use crate::state::{UserFee, VaultHeader};
use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;

// Fixed header of the dynamic fee vault. The account bytes are laid out as
// `[discriminator][DynamicFeeVault][UserFee; N]` and split at load time into a
// fixed part (`fixed`) and a growable `UserFee` tail via `d_load*` (see
// `utils::dynamic_loader`). `N` (2..=100) varies per account.
//
// The `VaultHeader` is shared with `FeeVault` so header utilities can be
// reused.
#[account(zero_copy)]
#[derive(InitSpace, Debug, Default)]
#[repr(C, align(8))] // align(8) so the trailing `UserFee` tail stays aligned
pub struct DynamicFeeVault {
    pub fixed: VaultHeader,
}
const_assert_eq!(DynamicFeeVault::INIT_SPACE, 240);
// Keep the `UserFee` tail 8-byte aligned after the discriminator + fixed header.
const_assert_eq!((8 + DynamicFeeVault::INIT_SPACE) % 8, 0);
const_assert_eq!(UserFee::INIT_SPACE % 8, 0);

impl std::ops::Deref for DynamicFeeVault {
    type Target = VaultHeader;
    fn deref(&self) -> &Self::Target {
        &self.fixed
    }
}

impl std::ops::DerefMut for DynamicFeeVault {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fixed
    }
}

impl DynamicFeeVault {
    pub fn space(num_users: usize) -> usize {
        8 + DynamicFeeVault::INIT_SPACE + num_users * UserFee::INIT_SPACE
    }

    pub fn initialize_header(
        &mut self,
        owner: &Pubkey,
        token_flag: u8,
        token_mint: &Pubkey,
        token_vault: &Pubkey,
        base: &Pubkey,
        vault_bump: u8,
        vault_type: u8,
    ) {
        self.fixed.initialize(
            owner,
            token_flag,
            token_mint,
            token_vault,
            base,
            vault_bump,
            vault_type,
        );
    }
}

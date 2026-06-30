use crate::{constants::MAX_FEE_VAULT_USER, math::SafeMath, params::UserShare, state::VaultHeader};
use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;

#[account(zero_copy)]
#[derive(InitSpace, Debug, Default)]
pub struct FeeVault {
    pub fixed: VaultHeader,
    pub users: [UserFee; MAX_FEE_VAULT_USER],
}
const_assert_eq!(FeeVault::INIT_SPACE, 640);

impl std::ops::Deref for FeeVault {
    type Target = VaultHeader;
    fn deref(&self) -> &Self::Target {
        &self.fixed
    }
}

impl std::ops::DerefMut for FeeVault {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fixed
    }
}

#[zero_copy]
#[derive(InitSpace, Debug, Default)]
pub struct UserFee {
    pub address: Pubkey,
    pub share: u32,
    pub padding_0: [u8; 4],
    pub fee_claimed: u64,
    pub padding: [u8; 16], // padding for future use
    pub fee_per_share_checkpoint: u128,
}
const_assert_eq!(UserFee::INIT_SPACE, 80);

impl FeeVault {
    pub fn initialize(
        &mut self,
        owner: &Pubkey,
        token_flag: u8,
        token_mint: &Pubkey,
        token_vault: &Pubkey,
        base: &Pubkey,
        vault_bump: u8,
        vault_type: u8,
        users: &[UserShare],
    ) -> Result<()> {
        self.fixed.initialize(
            owner,
            token_flag,
            token_mint,
            token_vault,
            base,
            vault_bump,
            vault_type,
        );

        let mut total_share = 0;
        for i in 0..users.len() {
            self.users[i] = UserFee {
                address: users[i].address,
                share: users[i].share,
                ..Default::default()
            };
            total_share = total_share.safe_add(users[i].share)?;
        }
        self.fixed.total_share = total_share;

        Ok(())
    }
}

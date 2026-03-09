use crate::math::SafeMath;
use anchor_lang::prelude::*;
use static_assertions::const_assert_eq;

#[account(zero_copy)]
#[derive(InitSpace, Debug, Default)]
pub struct UserUnclaimedFee {
    pub unclaimed_fee: u64,
    pub user: Pubkey,
    pub fee_vault: Pubkey,
    pub padding: [u8; 32], //  padding for future use
}

const_assert_eq!(UserUnclaimedFee::INIT_SPACE, 104);

impl UserUnclaimedFee {
    pub fn initialize(&mut self, user: Pubkey, fee_vault: Pubkey) {
        self.user = user;
        self.fee_vault = fee_vault;
    }

    pub fn add_unclaimed_fee(&mut self, unclaimed_fee: u64) -> Result<()> {
        self.unclaimed_fee = self.unclaimed_fee.safe_add(unclaimed_fee)?;
        Ok(())
    }
}

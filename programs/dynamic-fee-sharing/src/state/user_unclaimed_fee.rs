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
    pub fn initialize_and_add_unclaimed_fee(
        &mut self,
        user: Pubkey,
        fee_vault: Pubkey,
        unclaimed_fee: u64,
    ) -> Result<()> {
        self.user = user;
        self.fee_vault = fee_vault;
        self.unclaimed_fee = self.unclaimed_fee.safe_add(unclaimed_fee)?;
        Ok(())
    }
}

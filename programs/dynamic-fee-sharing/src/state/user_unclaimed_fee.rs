use crate::constants::seeds::USER_UNCLAIMED_FEE_PREFIX;
use crate::math::SafeMath;
use crate::utils::{
    account::create_pda_account_with_anchor_discriminator,
    dynamic_loader::{is_account_initialized, load_mut_checked, load_mut_unchecked},
};
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

    pub fn init_if_needed_and_add<'a>(
        user_unclaimed_fee: &AccountInfo<'a>,
        bump: u8,
        fee_vault: Pubkey,
        user: Pubkey,
        amount: u64,
        rent_payer: &AccountInfo<'a>,
        system_program: &AccountInfo<'a>,
    ) -> Result<()> {
        if !is_account_initialized::<UserUnclaimedFee>(user_unclaimed_fee)? {
            create_pda_account_with_anchor_discriminator::<UserUnclaimedFee>(
                rent_payer,
                system_program,
                user_unclaimed_fee,
                &[
                    USER_UNCLAIMED_FEE_PREFIX,
                    fee_vault.as_ref(),
                    user.as_ref(),
                    &[bump],
                ],
            )?;

            let mut user_unclaimed_fee =
                load_mut_unchecked::<UserUnclaimedFee, UserUnclaimedFee>(user_unclaimed_fee)?;
            user_unclaimed_fee.initialize(user, fee_vault);
            user_unclaimed_fee.add_unclaimed_fee(amount)?;
        } else {
            let mut user_unclaimed_fee =
                load_mut_checked::<UserUnclaimedFee, UserUnclaimedFee>(user_unclaimed_fee)?;
            user_unclaimed_fee.add_unclaimed_fee(amount)?;
        }
        Ok(())
    }
}

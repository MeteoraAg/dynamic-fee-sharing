use anchor_lang::prelude::*;

use crate::{
    constants::{MAX_USER, PRECISION_SCALE},
    error::FeeVaultError,
    instructions::UserShare,
    math::{mul_shr, shl_div, SafeMath},
};

#[account(zero_copy)]
#[derive(InitSpace, Debug, Default)]
pub struct FeeVault {
    pub owner: Pubkey,
    pub token_mint: Pubkey,
    pub token_vault: Pubkey,
    pub token_flag: u8, // indlicate whether token is spl-token or token2022
    pub padding_0: [u8; 15],
    pub total_share: u64,
    pub total_funded_fee: u64,
    pub fee_per_share: u128,
    pub padding: [u128; 6],
    pub users: [UserFee; MAX_USER],
}

#[zero_copy]
#[derive(InitSpace, Debug, Default)]
pub struct UserFee {
    pub address: Pubkey,
    pub share: u64,
    pub fee_pending: u64, // not used for not
    pub fee_claimed: u64,
    pub padding: [u8; 8],
    pub fee_per_share_checkpoint: u128,
}

impl FeeVault {
    pub fn initalize(
        &mut self,
        owner: &Pubkey,
        token_flag: u8,
        token_mint: &Pubkey,
        token_vault: &Pubkey,
        users: &[UserShare],
    ) -> Result<()> {
        self.owner = *owner;
        self.token_flag = token_flag;
        self.token_mint = *token_mint;
        self.token_vault = *token_vault;
        let mut total_share = 0;
        for i in 0..users.len() {
            self.users[i] = UserFee {
                address: users[i].address,
                share: users[i].share,
                ..Default::default()
            };
            total_share = total_share.safe_add(users[i].share)?;
        }
        self.total_share = total_share;
        Ok(())
    }

    pub fn fund_fee(&mut self, amount: u64) -> Result<()> {
        self.total_funded_fee = self.total_funded_fee.safe_add(amount)?;

        let fee_per_share = shl_div(amount, self.total_share, PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?;

        self.fee_per_share = self.fee_per_share.safe_add(fee_per_share)?;

        Ok(())
    }

    pub fn validate_and_claim_fee(&mut self, index: u8, signer: &Pubkey) -> Result<u64> {
        let user = self
            .users
            .get_mut(index as usize)
            .ok_or_else(|| FeeVaultError::InvalidUserIndex)?;
        require!(user.address.eq(signer), FeeVaultError::InvalidUserAddress);

        let rewad_per_share_delta = self.fee_per_share.safe_sub(user.fee_per_share_checkpoint)?;

        let fee_being_claimed = mul_shr(user.share.into(), rewad_per_share_delta, PRECISION_SCALE)
            .ok_or_else(|| FeeVaultError::MathOverflow)?
            .try_into()
            .map_err(|_| FeeVaultError::MathOverflow)?;

        user.fee_per_share_checkpoint = self.fee_per_share;
        user.fee_claimed = user.fee_claimed.safe_add(fee_being_claimed)?;

        Ok(fee_being_claimed)
    }
}

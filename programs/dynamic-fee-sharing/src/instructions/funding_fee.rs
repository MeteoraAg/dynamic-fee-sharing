use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

use crate::{
    error::FeeVaultError,
    event::EvtFundFee,
    math::SafeMath,
    state::{FeeVault, FundingType},
};
pub fn handle_funding_fee<'info, F: Fn(&[&[u8]; 4]) -> Result<()>>(
    signer: &Pubkey,
    fee_vault_account: &AccountLoader<'_, FeeVault>,
    token_b_account: &mut Box<InterfaceAccount<'info, TokenAccount>>,
    token_b_mint: &Pubkey,
    funder: Pubkey,
    funding_type: FundingType,
    op: F,
) -> Result<()> {
    let fee_vault = fee_vault_account.load()?;

    require!(
        fee_vault.is_share_holder(signer),
        FeeVaultError::InvalidSigner
    );

    require!(
        fee_vault.token_vault.eq(&token_b_account.key()) && fee_vault.token_mint.eq(&token_b_mint),
        FeeVaultError::InvalidFeeVault
    );

    // support fee vault type is pda account
    require!(
        fee_vault.fee_vault_type == 1,
        FeeVaultError::InvalidFeeVault
    );

    let signer_seeds = fee_vault_seeds!(
        fee_vault.base,
        fee_vault.token_mint,
        fee_vault.fee_vault_bump
    );

    let before_token_vault_balance = token_b_account.amount;

    op(signer_seeds)?;

    token_b_account.reload()?;

    let after_token_vault_balance = token_b_account.amount;

    let claimed_amount = after_token_vault_balance.safe_sub(before_token_vault_balance)?;

    if claimed_amount > 0 {
        drop(fee_vault);

        let mut fee_vault = fee_vault_account.load_mut()?;
        fee_vault.fund_fee(claimed_amount)?;

        emit!(EvtFundFee {
            funding_type,
            fee_vault: fee_vault_account.key(),
            funder,
            funded_amount: claimed_amount,
            fee_per_share: fee_vault.fee_per_share,
        });
    }

    Ok(())
}

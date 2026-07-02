use crate::constants::seeds::USER_UNCLAIMED_FEE_PREFIX;
use crate::error::FeeVaultError;
use crate::event::EvtRemoveUser;
use crate::state::{remove_user_and_shrink, DynamicFeeVault, UserUnclaimedFee};
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct RemoveUserCtx<'info> {
    #[account(mut, has_one = owner @ FeeVaultError::InvalidSigner)]
    pub fee_vault: AccountLoader<'info, DynamicFeeVault>,

    /// CHECK: the user being removed
    pub user: UncheckedAccount<'info>,

    /// CHECK: PDA for removed user's unclaimed fee. Created in handler only when unclaimed_fee > 0.
    #[account(
        mut,
        seeds = [
            USER_UNCLAIMED_FEE_PREFIX,
            fee_vault.key().as_ref(),
            user.key().as_ref(),
        ],
        bump,
    )]
    pub user_unclaimed_fee: UncheckedAccount<'info>,

    /// CHECK: receives excess rent lamports after account shrink. can be any address
    #[account(mut)]
    pub rent_receiver: UncheckedAccount<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_remove_user(ctx: Context<RemoveUserCtx>, index: u8) -> Result<()> {
    let user = ctx.accounts.user.key();
    let fee_vault_key = ctx.accounts.fee_vault.key();

    let (unclaimed_fee_0, unclaimed_fee_1) = remove_user_and_shrink(
        &ctx.accounts.fee_vault,
        &ctx.accounts.rent_receiver.to_account_info(),
        index.into(),
        &user,
    )?;

    if unclaimed_fee_0 > 0 || unclaimed_fee_1 > 0 {
        UserUnclaimedFee::init_if_needed_and_add(
            &ctx.accounts.user_unclaimed_fee.to_account_info(),
            ctx.bumps.user_unclaimed_fee,
            fee_vault_key,
            user,
            unclaimed_fee_0,
            unclaimed_fee_1,
            &ctx.accounts.owner.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
        )?;
    }

    emit_cpi!(EvtRemoveUser {
        fee_vault: fee_vault_key,
        user,
        unclaimed_fee_0,
        unclaimed_fee_1,
    });

    Ok(())
}

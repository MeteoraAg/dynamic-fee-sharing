use crate::constants::seeds::USER_UNCLAIMED_FEE_PREFIX;
use crate::event::EvtRemoveUser;
use crate::state::{remove_user_and_shrink_if_needed, FeeVault, UserUnclaimedFee};
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct RemoveUserCtx<'info> {
    #[account(mut)]
    pub fee_vault: AccountLoader<'info, FeeVault>,

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
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handle_remove_user(ctx: Context<RemoveUserCtx>, index: u8) -> Result<()> {
    let user = ctx.accounts.user.key();
    let fee_vault_key = ctx.accounts.fee_vault.key();

    let unclaimed_fee = remove_user_and_shrink_if_needed(
        &ctx.accounts.fee_vault,
        &ctx.accounts.rent_receiver.to_account_info(),
        index.into(),
        &user,
    )?;

    if unclaimed_fee > 0 {
        UserUnclaimedFee::init_if_needed_and_add(
            &ctx.accounts.user_unclaimed_fee.to_account_info(),
            ctx.bumps.user_unclaimed_fee,
            fee_vault_key,
            user,
            unclaimed_fee,
            &ctx.accounts.signer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
        )?;
    }

    emit_cpi!(EvtRemoveUser {
        fee_vault: fee_vault_key,
        user,
        unclaimed_fee,
    });

    Ok(())
}

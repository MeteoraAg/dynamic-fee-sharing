use crate::constants::seeds::USER_UNCLAIMED_FEE_PREFIX;
use crate::event::EvtRemoveUser;
use crate::state::{FeeVault, UserUnclaimedFee};
use crate::utils::account::{
    create_pda_account_with_anchor_discriminator, validate_and_load_account_data_mut,
};
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

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handle_remove_user(ctx: Context<RemoveUserCtx>, index: u8) -> Result<()> {
    let mut fee_vault = ctx.accounts.fee_vault.load_mut()?;
    let user = ctx.accounts.user.key();
    let unclaimed_fee =
        fee_vault.validate_and_remove_user_and_get_unclaimed_fee(index.into(), &user)?;

    if unclaimed_fee > 0 {
        let user_unclaimed_fee_account = &ctx.accounts.user_unclaimed_fee;
        let fee_vault_key = ctx.accounts.fee_vault.key();

        if user_unclaimed_fee_account.data_is_empty() {
            let bump = ctx.bumps.user_unclaimed_fee;

            create_pda_account_with_anchor_discriminator::<UserUnclaimedFee>(
                &ctx.accounts.signer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                &user_unclaimed_fee_account.to_account_info(),
                &[
                    USER_UNCLAIMED_FEE_PREFIX,
                    fee_vault_key.as_ref(),
                    user.as_ref(),
                    &[bump],
                ],
            )?;
        }

        let mut data = user_unclaimed_fee_account.try_borrow_mut_data()?;
        let user_unclaimed_fee = validate_and_load_account_data_mut::<UserUnclaimedFee>(
            user_unclaimed_fee_account.owner,
            &mut data,
        )?;
        user_unclaimed_fee.initialize_and_add_unclaimed_fee(user, fee_vault_key, unclaimed_fee)?;
    }

    emit_cpi!(EvtRemoveUser {
        fee_vault: ctx.accounts.fee_vault.key(),
        user,
        unclaimed_fee,
    });

    Ok(())
}

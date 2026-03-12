use crate::constants::seeds::USER_UNCLAIMED_FEE_PREFIX;
use crate::event::EvtRemoveUser;
use crate::state::{shrink_dynamic_user, DynamicFeeVaultLoader, FeeVault, UserUnclaimedFee};
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

    /// CHECK: receives excess rent lamports after account shrink. can be any address
    #[account(mut)]
    pub rent_receiver: UncheckedAccount<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handle_remove_user(ctx: Context<RemoveUserCtx>, index: u8) -> Result<()> {
    let fee_vault_info = ctx.accounts.fee_vault.as_ref().to_account_info();
    let user = ctx.accounts.user.key();

    let (unclaimed_fee, should_shrink) = {
        let mut vault = ctx.accounts.fee_vault.load_content_mut()?;
        vault.remove_user(index.into(), &user)?
    };

    if unclaimed_fee > 0 {
        let user_unclaimed_fee_account = &ctx.accounts.user_unclaimed_fee;
        let fee_vault_key = ctx.accounts.fee_vault.key();

        let is_empty = user_unclaimed_fee_account.data_is_empty();
        if is_empty {
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
        if is_empty {
            user_unclaimed_fee.initialize(user, fee_vault_key);
        }
        user_unclaimed_fee.add_unclaimed_fee(unclaimed_fee)?;
    }

    if should_shrink {
        shrink_dynamic_user(
            &fee_vault_info,
            &ctx.accounts.rent_receiver.to_account_info(),
        )?;
    }

    emit_cpi!(EvtRemoveUser {
        fee_vault: ctx.accounts.fee_vault.key(),
        user,
        unclaimed_fee,
    });

    Ok(())
}

use anchor_lang::prelude::*;
#[macro_use]
pub mod macros;
pub mod constants;
pub mod error;
pub mod instructions;
pub use instructions::*;
pub mod access_control;
pub mod const_pda;
pub mod event;
pub mod math;
pub mod state;
pub mod utils;
pub use access_control::*;
use state::OperatorPermission;

pub mod tests;
declare_id!("dfsdo2UqvwfN8DuUVrMRNfQe11VaiNoKcMqLHVvDPzh");

#[program]
pub mod dynamic_fee_sharing {
    use super::*;
    pub fn initialize_fee_vault(
        ctx: Context<InitializeFeeVaultCtx>,
        params: InitializeFeeVaultParameters,
    ) -> Result<()> {
        instructions::handle_initialize_fee_vault(ctx, &params)
    }

    pub fn initialize_fee_vault_pda(
        ctx: Context<InitializeFeeVaultPdaCtx>,
        params: InitializeFeeVaultParameters,
    ) -> Result<()> {
        instructions::handle_initialize_fee_vault_pda(ctx, &params)
    }

    pub fn fund_fee(ctx: Context<FundFeeCtx>, max_amount: u64) -> Result<()> {
        instructions::handle_fund_fee(ctx, max_amount)
    }

    pub fn fund_by_claiming_fee(
        ctx: Context<FundByClaimingFeeCtx>,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::handle_fund_by_claiming_fee(ctx, payload)
    }

    pub fn claim_fee(ctx: Context<ClaimFeeCtx>, index: u8) -> Result<()> {
        instructions::handle_claim_fee(ctx, index)
    }

    #[access_control(is_valid_operator_role(&ctx.accounts.fee_vault, &ctx.accounts.operator, ctx.accounts.signer.key, OperatorPermission::UpdateUserShare))]
    pub fn update_user_share(
        ctx: Context<UpdateUserShareCtx>,
        index: u8,
        share: u32,
    ) -> Result<()> {
        instructions::handle_update_user_share(ctx, index, share)
    }

    pub fn create_operator_account(
        ctx: Context<CreateOperatorAccountCtx>,
        permission: u128,
    ) -> Result<()> {
        instructions::handle_create_operator_account(ctx, permission)
    }

    pub fn close_operator_account(_ctx: Context<CloseOperatorAccountCtx>) -> Result<()> {
        Ok(())
    }
}

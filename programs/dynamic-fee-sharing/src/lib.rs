use anchor_lang::prelude::*;
#[macro_use]
pub mod macros;
pub mod constants;
pub mod error;
pub mod instructions;
pub use instructions::*;
pub mod const_pda;
pub mod event;
pub mod math;
pub mod state;
pub mod utils;

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

    pub fn update_operator(ctx: Context<UpdateOperatorAccountCtx>) -> Result<()> {
        instructions::handle_update_operator(ctx)
    }

    pub fn update_user_share(
        ctx: Context<UpdateUserShareCtx>,
        index: u8,
        share: u32,
    ) -> Result<()> {
        instructions::handle_update_user_share(ctx, index, share)
    }
}

use anchor_lang::prelude::*;
#[macro_use]
pub mod macros;
pub mod access_control;
use access_control::*;
pub mod constants;
pub mod error;
pub mod instructions;
pub use instructions::*;
pub mod params;
pub use params::{CreateWhitelistedActionParameters, InitializeFeeVaultParameters};
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
    /// Accepts: FeeVault only.
    pub fn initialize_fee_vault(
        ctx: Context<InitializeFeeVaultCtx>,
        params: InitializeFeeVaultParameters,
    ) -> Result<()> {
        instructions::handle_initialize_fee_vault(ctx, &params)
    }

    /// Accepts: FeeVault only.
    pub fn initialize_fee_vault_pda(
        ctx: Context<InitializeFeeVaultPdaCtx>,
        params: InitializeFeeVaultParameters,
    ) -> Result<()> {
        instructions::handle_initialize_fee_vault_pda(ctx, &params)
    }

    /// Accepts: DynamicFeeVault only.
    pub fn initialize_dynamic_fee_vault(
        ctx: Context<InitializeDynamicFeeVaultCtx>,
        params: InitializeFeeVaultParameters,
    ) -> Result<()> {
        instructions::handle_initialize_dynamic_fee_vault(ctx, &params)
    }

    /// Accepts: DynamicFeeVault only.
    pub fn initialize_dynamic_fee_vault_pda(
        ctx: Context<InitializeDynamicFeeVaultPdaCtx>,
        params: InitializeFeeVaultParameters,
    ) -> Result<()> {
        instructions::handle_initialize_dynamic_fee_vault_pda(ctx, &params)
    }

    /// Accepts: FeeVault or DynamicFeeVault.
    pub fn fund_fee(ctx: Context<FundFeeCtx>, max_amount: u64) -> Result<()> {
        instructions::handle_fund_fee(ctx, max_amount)
    }

    /// Accepts: FeeVault only.
    pub fn fund_by_claiming_fee(
        ctx: Context<FundByClaimingFeeCtx>,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::handle_fund_by_claiming_fee(ctx, payload)
    }

    /// Accepts: FeeVault or DynamicFeeVault.
    pub fn claim_fee(ctx: Context<ClaimFeeCtx>, index: u8) -> Result<()> {
        instructions::handle_claim_fee(ctx, index)
    }

    /// Accepts: DynamicFeeVault only.
    pub fn add_user(ctx: Context<AddUserCtx>, share: u32) -> Result<()> {
        instructions::handle_add_user(ctx, share)
    }

    /// Accepts: DynamicFeeVault only.
    pub fn update_user_share(
        ctx: Context<UpdateUserShareCtx>,
        index: u8,
        share: u32,
    ) -> Result<()> {
        instructions::handle_update_user_share(ctx, index, share)
    }

    /// Accepts: DynamicFeeVault only.
    pub fn remove_user(ctx: Context<RemoveUserCtx>, index: u8) -> Result<()> {
        instructions::handle_remove_user(ctx, index)
    }

    /// Accepts: DynamicFeeVault only.
    pub fn claim_unclaimed_fee(ctx: Context<ClaimUnclaimedFeeCtx>) -> Result<()> {
        instructions::handle_claim_unclaimed_fee(ctx)
    }

    #[access_control(is_admin(ctx.accounts.admin.key))]
    pub fn create_whitelisted_action(
        ctx: Context<CreateWhitelistedActionCtx>,
        params: CreateWhitelistedActionParameters,
    ) -> Result<()> {
        instructions::handle_create_whitelisted_action(ctx, &params)
    }

    #[access_control(is_admin(ctx.accounts.admin.key))]
    pub fn close_whitelisted_action(ctx: Context<CloseWhitelistedActionCtx>) -> Result<()> {
        instructions::handle_close_whitelisted_action(ctx)
    }

    /// Accepts: DynamicFeeVault only.
    pub fn fund_by_whitelisted_action(
        ctx: Context<FundByWhitelistedActionCtx>,
        discriminator: [u8; 8],
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::handle_fund_by_whitelisted_action(ctx, discriminator, payload)
    }
}

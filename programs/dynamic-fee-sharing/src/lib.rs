#![allow(unexpected_cfgs)]

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

    pub fn funding_by_claim_dammv2_fee(ctx: Context<FundingByClaimDammv2FeeCtx>) -> Result<()> {
        instructions::handle_funding_by_claim_dammv2_fee(ctx)
    }

    pub fn funding_by_claim_dbc_partner_trading_fee(
        ctx: Context<FundingByClaimDbcTradingFeeCtx>,
    ) -> Result<()> {
        instructions::handle_funding_by_claim_dbc_partner_trading_fee(ctx)
    }

    pub fn funding_by_claim_dbc_creator_trading_fee(
        ctx: Context<FundingByClaimDbcCreatorTradingFeeCtx>,
    ) -> Result<()> {
        instructions::handle_funding_by_claim_dbc_creator_trading_fee(ctx)
    }

    pub fn funding_by_claim_dbc_creator_surplus(
        ctx: Context<FundingByClaimDbcCreatorSurplusCtx>,
    ) -> Result<()> {
        instructions::handle_funding_by_claim_dbc_creator_surplus(ctx)
    }

    pub fn funding_by_claim_dbc_partner_surplus(
        ctx: Context<FundingByClaimDbcPartnerSurplusCtx>,
    ) -> Result<()> {
        instructions::handle_funding_by_claim_dbc_partner_surplus(ctx)
    }

    pub fn claim_fee(ctx: Context<ClaimFeeCtx>, index: u8) -> Result<()> {
        instructions::handle_claim_fee(ctx, index)
    }
}

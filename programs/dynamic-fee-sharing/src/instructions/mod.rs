// `#[event_cpi]` adds `AccountInfo` fields, which anchor 1.0.2 itself flags as deprecated in the
// generated `Accounts` impl
#[allow(deprecated)]
pub mod ix_initialize_fee_vault;
pub use ix_initialize_fee_vault::*;
#[allow(deprecated)]
pub mod ix_fund_fee;
pub use ix_fund_fee::*;
#[allow(deprecated)]
pub mod ix_claim_fee;
pub use ix_claim_fee::*;
#[allow(deprecated)]
pub mod ix_initialize_fee_vault_pda;
pub use ix_initialize_fee_vault_pda::*;
#[allow(deprecated)]
pub mod ix_fund_by_claiming_fee;
pub use ix_fund_by_claiming_fee::*;

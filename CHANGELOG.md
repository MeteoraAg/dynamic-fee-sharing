# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

### Breaking Changes

## dynamic-fee-sharing [0.1.1] [PR #8](https://github.com/MeteoraAg/dynamic-fee-sharing/pull/8)

### Added
- Add new field `fee_vault_type` in `FeeVault` to distinguish between PDA-derived and keypair-derived fee vaults.
- Permissionless Funding Endpoints (supporting PDA account fee vaults):
    - New endpoint `ix_funding_by_claim_damm_v2` allows funding by claiming Damm-V2 position fee.
    - New endpoint `ix_funding_by_claim_dbc_creator_surplus` allows funding by claiming DBC creator surplus.
    - New endpoint `ix_funding_by_claim_dbc_partner_surplus` allows funding by claiming DBC partner surplus.
    - New endpoint `ix_funding_by_claim_dbc_creator_trading_fee` allows funding by claiming DBC creator trading fee.
    - New endpoint `ix_funding_by_claim_dbc_partner_trading_fee` allows funding by claiming DBC partner trading fee.


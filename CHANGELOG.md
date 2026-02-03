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

## dynamic-fee-sharing [0.1.2] [PR #15](https://github.com/MeteoraAg/dynamic-fee-sharing/pull/15)

### Added

- Add a new endpoint `create_operator_account` and `close_operator_account`that allows vault owner to manage different operator accounts
- Add a new account `Operator`, that would stores `whitelisted_address` as well as their operational permissions
- Add a new endpoint `update_user_share` that allows an operator to update a user's share. This affects the fees the user will be entitled to when the vault is funded. Any fees users earned before the share changed will be preserved.

### Changed

- Update anchor to `1.0.2`

## dynamic-fee-sharing [0.1.1] [PR #8](https://github.com/MeteoraAg/dynamic-fee-sharing/pull/8)

### Added

- Add new field `fee_vault_type` in `FeeVault` to distinguish between PDA-derived and keypair-derived fee vaults.
- Add new endpoint `fund_by_claiming_fee`, that allow share holder in fee vault to claim fees from whitelisted endpoints of DAMM-v2 or Dynamic Bonding Curve

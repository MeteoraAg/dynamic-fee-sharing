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

- Add a new field `mutable_flag` to `FeeVault` to indicate its mutability
- Add a new field `operator` to `FeeVault`. The `operator` defaults to the owner when the `FeeVault` is initialized. The `operator` and can perform operator instructions on a mutable `FeeVault`
- Add a new owner endpoint `update_operator` for vault owner to update the operator field
- Add a new operator endpoint `add_user` to add a user to a `FeeVault`
- Add a new operator endpoint `remove_user` which removes a user and transfers any unclaimed fee into an account for the removed user to claim
- Add a new endpoint `claim_unclaimed_fee` where a user who have been removed from the `FeeVault` can claim any unclaimed fees
- Add a new operator endpoint `update_user_share` to update a user's share. This affects the fees the user will be entitled to when the vault is funded. Any fees users earned before the share changed will be preserved
- Increase the `MAX_USER` limit from 5 to 100

### Changed

- Update anchor to `1.0.2`

### Breaking Changes

- Prevent initializing a `FeeVault` with duplicate user address. This change affects both `initialize_fee_vault` and `initialize_fee_vault_pda` endpoints.
- Rename error code `ExceededUser` to `InvalidNumberOfUsers`

## dynamic-fee-sharing [0.1.1] [PR #8](https://github.com/MeteoraAg/dynamic-fee-sharing/pull/8)

### Added

- Add new field `fee_vault_type` in `FeeVault` to distinguish between PDA-derived and keypair-derived fee vaults.
- Add new endpoint `fund_by_claiming_fee`, that allow share holder in fee vault to claim fees from whitelisted endpoints of DAMM-v2 or Dynamic Bonding Curve

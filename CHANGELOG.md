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

## dynamic-fee-sharing [0.1.3]

### Added

- New endpoint `update_user_address(index, new_address)`: the user at `index` moves its share to `new_address`, signed by its current address. The share and the fee not claimed yet stay with the slot, so they go to the new address. It emits `EvtUpdateUserAddress`. Only the current address can sign, so it is meant to rotate a wallet, not to recover from a compromised key: whoever holds that key can update the address too

### Changed

- Update to anchor 1.2.1 and solana 4.1.2, building for SBPF v3 with platform-tools v1.57

## dynamic-fee-sharing [0.1.2] [PR #15](https://github.com/MeteoraAg/dynamic-fee-sharing/pull/15)

### Added

- Whitelist DBC `claim_trading_fee2` and `claim_creator_trading_fee2` in `fund_by_claiming_fee`

### Changed

- Update anchor to `1.0.2`
- The `fund_by_claiming_fee` endpoint now rejects when `skip_reward` is non-zero for DAMM v2 `claim_reward` instruction to prevent clearing pending rewards without funding the `FeeVault`

## dynamic-fee-sharing [0.1.1] [PR #8](https://github.com/MeteoraAg/dynamic-fee-sharing/pull/8)

### Added

- Add new field `fee_vault_type` in `FeeVault` to distinguish between PDA-derived and keypair-derived fee vaults.
- Add new endpoint `fund_by_claiming_fee`, that allow share holder in fee vault to claim fees from whitelisted endpoints of DAMM-v2 or Dynamic Bonding Curve

use anchor_lang::prelude::*;

use crate::{
    error::FeeVaultError,
    state::{FeeVault, UserFee},
};

fn fee_vault_with_users(addresses: &[Pubkey]) -> FeeVault {
    let mut fee_vault = FeeVault {
        total_share: 2_000,
        ..Default::default()
    };
    for (i, address) in addresses.iter().enumerate() {
        fee_vault.users[i] = UserFee {
            address: *address,
            share: 1_000,
            ..Default::default()
        };
    }
    fee_vault
}

#[test]
fn test_update_user_address_keeps_the_pending_fee_with_the_slot() {
    let (old_address, other, new_address) = (
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    );
    let mut fee_vault = fee_vault_with_users(&[old_address, other]);
    fee_vault.fund_fee(1_000).unwrap();

    // claim once, then fund again so some fee is pending
    let claimed_before = fee_vault.validate_and_claim_fee(0, &old_address).unwrap();
    fee_vault.fund_fee(1_000).unwrap();

    let returned = fee_vault
        .update_user_address(0, &old_address, &new_address)
        .unwrap();
    assert_eq!(returned, old_address);
    assert_eq!(fee_vault.users[0].address, new_address);
    assert_eq!(fee_vault.users[0].share, 1_000);
    assert_eq!(fee_vault.users[0].fee_claimed, claimed_before);

    // the old address cannot claim anymore, the new one gets the pending fee
    assert_eq!(
        fee_vault
            .validate_and_claim_fee(0, &old_address)
            .unwrap_err(),
        FeeVaultError::InvalidUserAddress.into()
    );
    assert_eq!(
        fee_vault.validate_and_claim_fee(0, &new_address).unwrap(),
        500
    );
    // the other user is untouched
    assert_eq!(fee_vault.users[1].address, other);
}

#[test]
fn test_update_user_address_rejects_a_signer_that_is_not_the_user() {
    let (address, other) = (Pubkey::new_unique(), Pubkey::new_unique());
    let mut fee_vault = fee_vault_with_users(&[address, other]);

    // another share holder cannot move someone else's slot
    assert_eq!(
        fee_vault
            .update_user_address(0, &other, &Pubkey::new_unique())
            .unwrap_err(),
        FeeVaultError::InvalidUserAddress.into()
    );
    assert_eq!(fee_vault.users[0].address, address);
}

#[test]
fn test_update_user_address_rejects_an_invalid_new_address_or_index() {
    let (address, other) = (Pubkey::new_unique(), Pubkey::new_unique());
    let mut fee_vault = fee_vault_with_users(&[address, other]);

    for new_address in [Pubkey::default(), address] {
        assert_eq!(
            fee_vault
                .update_user_address(0, &address, &new_address)
                .unwrap_err(),
            FeeVaultError::InvalidUserAddress.into()
        );
    }
    assert_eq!(
        fee_vault
            .update_user_address(u8::MAX, &address, &Pubkey::new_unique())
            .unwrap_err(),
        FeeVaultError::InvalidUserIndex.into()
    );
    assert_eq!(fee_vault.users[0].address, address);
}

use std::cell::RefCell;
use std::u32;

use crate::state::{FeeVault, VaultHeader, VaultOps};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10000, .. ProptestConfig::default()
    })]

    #[test]
    fn test_fund_fee_small_amount_wont_loss_precision(amount in 1..=10000u64) {
        let fee_vault = RefCell::new(FeeVault {
            fixed: VaultHeader {
                total_share: u32::MAX,
                ..Default::default()
            },
            ..Default::default()
        });

        fee_vault.borrow_mut().fund_fee(true, amount).unwrap();

        assert!(fee_vault.borrow().fee_per_share > 0);
    }
}

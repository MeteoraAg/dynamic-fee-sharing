use std::u32;

use crate::state::FeeVault;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10000, .. ProptestConfig::default()
    })]

    #[test]
    fn test_fund_fee_small_amount_wont_loss_precision(amount in 1..=10000u64) {
        let mut fee_vault = FeeVault {
            total_share: u32::MAX,
            ..Default::default()
        };

        fee_vault.fund_fee(amount).unwrap();

        assert!(fee_vault.fee_per_share > 0);
    }
}

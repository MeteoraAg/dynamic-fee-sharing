use crate::{
    constants::MAX_OPERATION,
    state::operator::{Operator, OperatorPermission},
};

#[test]
fn test_initialize_with_full_permission() {
    let permission: u128 = 0b1;
    assert!(permission >= 1 << (MAX_OPERATION - 1) && permission <= 1 << MAX_OPERATION);

    let operator = Operator {
        permission,
        ..Default::default()
    };

    assert_eq!(
        operator.is_permission_allow(OperatorPermission::UpdateUserShare),
        true
    );
}

#[test]
fn test_is_permission_not_allow() {
    let operator = Operator {
        permission: 0b0,
        ..Default::default()
    };
    assert_eq!(
        operator.is_permission_allow(OperatorPermission::UpdateUserShare),
        false
    );
}

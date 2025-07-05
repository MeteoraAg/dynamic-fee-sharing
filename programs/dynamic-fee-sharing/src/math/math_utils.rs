use ruint::aliases::U256;

/// (x << offset) / y
#[inline]
pub fn shl_div(x: u64, y: u64, offset: u8) -> Option<u128> {
    if y == 0 {
        return None;
    }
    let denominator = u128::from(y);
    let prod = u128::from(x).checked_shl(offset as u32)?;
    let result = prod.checked_div(denominator)?;
    Some(result)
}

#[inline]
pub fn mul_shr(x: u128, y: u128, offset: u8) -> Option<u128> {
    let x = U256::from(x);
    let y = U256::from(y);
    let prod = x.checked_mul(y)?;
    let (quotient, _is_overflow) = prod.overflowing_shr(offset.into());
    quotient.try_into().ok()
}

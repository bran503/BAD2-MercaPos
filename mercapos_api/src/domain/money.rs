//! Exact decimal parsing for the public JSON contract. No binary float is used
//! before validating quantities, prices and totals.

pub fn parse_scaled(input: &str, digits: usize, max_abs: i64) -> Result<i64, String> {
    let (negative, unsigned) = match input.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, input),
    };
    let mut split = unsigned.split('.');
    let whole = split.next().unwrap_or("");
    let fraction = split.next().unwrap_or("");
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > digits
        || split.next().is_some()
    {
        return Err("Número decimal o precisión inválida".into());
    }
    let scale = 10_i128.pow(u32::try_from(digits).map_err(|_| "Escala inválida")?);
    let whole: i128 = whole.parse().map_err(|_| "Número fuera de rango")?;
    let fraction_len = fraction.len();
    let fraction_value: i128 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().map_err(|_| "Número fuera de rango")?
    };
    let fraction_scale =
        10_i128.pow(u32::try_from(digits - fraction_len).map_err(|_| "Escala inválida")?);
    let magnitude = whole
        .checked_mul(scale)
        .and_then(|n| n.checked_add(fraction_value * fraction_scale))
        .ok_or("Número fuera de rango")?;
    if magnitude > i128::from(max_abs) {
        return Err("Número fuera de rango".into());
    }
    let signed = if negative { -magnitude } else { magnitude };
    i64::try_from(signed).map_err(|_| "Número fuera de rango".into())
}

/// Quantity has three decimals, unit price four decimals, result is cents.
pub fn subtotal_centavos(cantidad_mil: i64, precio_diezmil: i64) -> Result<i64, String> {
    let product = i128::from(cantidad_mil)
        .checked_mul(i128::from(precio_diezmil))
        .ok_or("Importe fuera de rango")?;
    let rounded = product
        .checked_add(50_000)
        .ok_or("Importe fuera de rango")?
        / 100_000;
    i64::try_from(rounded).map_err(|_| "Importe fuera de rango".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn importes_exactos_y_precision() {
        assert_eq!(parse_scaled("1.25", 4, 10_000_000).unwrap(), 12_500);
        assert_eq!(subtotal_centavos(2_000, 12_500).unwrap(), 250);
        assert_eq!(subtotal_centavos(1_000, 3_350).unwrap(), 34);
        assert!(parse_scaled("1.0001", 3, 10_000_000).is_err());
        assert!(parse_scaled("1e3", 3, 10_000_000).is_err());
    }
}

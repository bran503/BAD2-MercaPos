/// Strict subset of RFC 3339 used by the API: seconds and an explicit zone.
pub fn valid_rfc3339(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return false;
    }
    let number = |start: usize, end: usize| -> Option<u32> {
        let part = bytes.get(start..end)?;
        if !part.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(part).ok()?.parse().ok()
    };
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0, 4),
        number(5, 7),
        number(8, 10),
        number(11, 13),
        number(14, 16),
        number(17, 19),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    if year == 0 || day == 0 || day > days || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let mut offset = 19;
    if bytes.get(offset) == Some(&b'.') {
        offset += 1;
        let begin = offset;
        while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
            offset += 1;
        }
        if offset == begin || offset - begin > 9 {
            return false;
        }
    }
    if bytes.get(offset) == Some(&b'Z') {
        return offset + 1 == bytes.len();
    }
    if !matches!(bytes.get(offset), Some(b'+' | b'-'))
        || bytes.get(offset + 3) != Some(&b':')
        || offset + 6 != bytes.len()
    {
        return false;
    }
    let (Some(zone_hour), Some(zone_minute)) = (
        number(offset + 1, offset + 3),
        number(offset + 4, offset + 6),
    ) else {
        return false;
    };
    zone_hour <= 23 && zone_minute <= 59
}

pub fn ahora_epoch_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::valid_rfc3339;
    #[test]
    fn fecha_y_zona_obligatorias() {
        assert!(valid_rfc3339("2026-11-01T10:00:00-06:00"));
        assert!(valid_rfc3339("2024-02-29T16:00:00.123Z"));
        assert!(!valid_rfc3339("2026-11-01T10:00:00"));
        assert!(!valid_rfc3339("2026-02-29T10:00:00Z"));
        assert!(!valid_rfc3339("2026-11-01"));
    }
}

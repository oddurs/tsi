//! Formatting utilities for unit display.

/// Format an integer with thousands separators (commas).
pub fn format_thousands(n: i64) -> String {
    if n == 0 {
        return "0".to_string();
    }

    let negative = n < 0;
    let mut n = n.unsigned_abs();
    let mut parts = Vec::new();

    while n > 0 {
        parts.push(format!("{:03}", n % 1000));
        n /= 1000;
    }

    parts.reverse();

    // Remove leading zeros from first part
    if let Some(first) = parts.first_mut() {
        *first = first.trim_start_matches('0').to_string();
        if first.is_empty() {
            *first = "0".to_string();
        }
    }

    let result = parts.join(",");
    if negative {
        format!("-{}", result)
    } else {
        result
    }
}

/// Format a float as an integer with thousands separators.
pub fn format_thousands_f64(n: f64) -> String {
    format_thousands(n.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_small_numbers() {
        assert_eq!(format_thousands(0), "0");
        assert_eq!(format_thousands(1), "1");
        assert_eq!(format_thousands(12), "12");
        assert_eq!(format_thousands(123), "123");
        assert_eq!(format_thousands(999), "999");
    }

    #[test]
    fn format_thousands_separator() {
        assert_eq!(format_thousands(1000), "1,000");
        assert_eq!(format_thousands(1234), "1,234");
        assert_eq!(format_thousands(12345), "12,345");
        assert_eq!(format_thousands(123456), "123,456");
        assert_eq!(format_thousands(1234567), "1,234,567");
    }

    #[test]
    fn format_large_numbers() {
        assert_eq!(format_thousands(1_000_000), "1,000,000");
        assert_eq!(format_thousands(142_300), "142,300");
        assert_eq!(format_thousands(9_400), "9,400");
    }

    #[test]
    fn format_negative_numbers() {
        assert_eq!(format_thousands(-1000), "-1,000");
        assert_eq!(format_thousands(-123456), "-123,456");
    }

    #[test]
    fn format_float() {
        assert_eq!(format_thousands_f64(142300.0), "142,300");
        assert_eq!(format_thousands_f64(9400.5), "9,401");
        assert_eq!(format_thousands_f64(999.4), "999");
    }
}

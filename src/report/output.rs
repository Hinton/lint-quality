//! Report output formatters (human-readable terminal, JSON, and interactive TUI).

pub mod human;
pub mod json;
pub mod tui;

/// Format a number with thousands separators (e.g. `1234567` → `"1,234,567"`).
pub fn fmt_num(n: usize) -> String {
    let s = n.to_string();
    if s.len() <= 3 {
        return s;
    }
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    let split = match s.len() % 3 { 0 => 3, r => r };
    result.push_str(&s[..split]);
    for chunk in s[split..].as_bytes().chunks(3) {
        result.push(',');
        result.push_str(std::str::from_utf8(chunk).unwrap());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_num_below_1000() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(1), "1");
        assert_eq!(fmt_num(999), "999");
    }

    #[test]
    fn fmt_num_4_digits() {
        assert_eq!(fmt_num(1_000), "1,000");
        assert_eq!(fmt_num(1_234), "1,234");
    }

    #[test]
    fn fmt_num_5_digits() {
        assert_eq!(fmt_num(12_345), "12,345");
    }

    #[test]
    fn fmt_num_6_digits() {
        assert_eq!(fmt_num(123_456), "123,456");
    }

    #[test]
    fn fmt_num_7_digits() {
        assert_eq!(fmt_num(1_234_567), "1,234,567");
    }

    #[test]
    fn fmt_num_boundary() {
        // 999 and 1_000 sit either side of the comma-insertion threshold
        assert_eq!(fmt_num(999).len(), 3);
        assert_eq!(fmt_num(1_000).len(), 5);
    }
}

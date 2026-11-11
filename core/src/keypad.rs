//! The on-screen number pad used to type amounts in the editor.

use crate::format::MAX_AMOUNT_DIGITS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Digit(u8),
    /// Appends "000": kip amounts are almost always whole thousands.
    TripleZero,
    Backspace,
    Clear,
}

/// Applies one key press to the amount being typed (`None` = nothing typed yet).
/// Presses that would exceed [`MAX_AMOUNT_DIGITS`] digits are ignored.
pub fn press_key(amount: Option<i64>, key: Key) -> Option<i64> {
    let current = amount.unwrap_or(0).max(0);
    let next = match key {
        Key::Digit(d) => append(current, i64::from(d.min(9)), 1),
        Key::TripleZero => append(current, 0, 3),
        Key::Backspace => current / 10,
        Key::Clear => 0,
    };
    (next > 0).then_some(next)
}

/// Shifts `current` left by `width` decimal digits and adds `tail`.
fn append(current: i64, tail: i64, width: u32) -> i64 {
    if current == 0 {
        return tail;
    }
    if digit_count(current) + width as usize > MAX_AMOUNT_DIGITS {
        return current;
    }
    current * 10_i64.pow(width) + tail
}

fn digit_count(n: i64) -> usize {
    n.to_string().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_keys(keys: &[Key]) -> Option<i64> {
        keys.iter().fold(None, |acc, k| press_key(acc, *k))
    }

    #[test]
    fn digits_build_the_number() {
        assert_eq!(type_keys(&[Key::Digit(4), Key::Digit(5)]), Some(45));
    }

    #[test]
    fn triple_zero_appends_thousands() {
        assert_eq!(type_keys(&[Key::Digit(4), Key::Digit(5), Key::TripleZero]), Some(45_000));
    }

    #[test]
    fn leading_zeros_are_ignored() {
        assert_eq!(press_key(None, Key::Digit(0)), None);
        assert_eq!(press_key(None, Key::TripleZero), None);
        assert_eq!(type_keys(&[Key::Digit(0), Key::TripleZero, Key::Digit(7)]), Some(7));
    }

    #[test]
    fn backspace_removes_the_last_digit() {
        assert_eq!(press_key(Some(45_000), Key::Backspace), Some(4_500));
        assert_eq!(press_key(Some(4), Key::Backspace), None);
        assert_eq!(press_key(None, Key::Backspace), None);
    }

    #[test]
    fn clear_empties_the_amount() {
        assert_eq!(press_key(Some(1_250_000), Key::Clear), None);
    }

    #[test]
    fn length_is_capped() {
        let full = Some(123_456_789_012); // 12 digits
        assert_eq!(press_key(full, Key::Digit(3)), full);
        assert_eq!(press_key(Some(1_234_567_890), Key::TripleZero), Some(1_234_567_890));
        assert_eq!(press_key(Some(123_456_789), Key::TripleZero), Some(123_456_789_000));
    }

    #[test]
    fn editing_an_existing_amount_appends() {
        assert_eq!(press_key(Some(45_000), Key::Digit(0)), Some(450_000));
    }
}

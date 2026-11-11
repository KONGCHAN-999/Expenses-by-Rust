//! Lao-language display formatting for kip amounts and dates.

use crate::date::{Date, YearMonth};
use crate::model::Kind;

/// Inputs longer than this are cut off (999,999,999,999 ₭ is plenty).
pub const MAX_AMOUNT_DIGITS: usize = 12;

const MONTHS: [&str; 12] = [
    "ມັງກອນ", "ກຸມພາ", "ມີນາ", "ເມສາ", "ພຶດສະພາ", "ມິຖຸນາ",
    "ກໍລະກົດ", "ສິງຫາ", "ກັນຍາ", "ຕຸລາ", "ພະຈິກ", "ທັນວາ",
];

/// `1234567` -> `"1,234,567"`.
pub fn format_number(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if n < 0 {
        out.push('-');
    }
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// `45000` -> `"45,000 ₭"`.
pub fn format_kip(n: i64) -> String {
    format!("{} ₭", format_number(n))
}

/// Expense -> `"-45,000 ₭"`, income -> `"+45,000 ₭"`.
pub fn signed_kip(kind: Kind, amount: i64) -> String {
    let sign = match kind {
        Kind::Expense => '-',
        Kind::Income => '+',
    };
    format!("{sign}{}", format_kip(amount.abs()))
}

/// A net amount (income minus expense): `-145,000 ₭` / `+1,500,000 ₭` / `0 ₭`.
pub fn net_kip(net: i64) -> String {
    match net {
        0 => format_kip(0),
        n if n < 0 => signed_kip(Kind::Expense, n),
        n => signed_kip(Kind::Income, n),
    }
}

pub fn month_name(month: u32) -> &'static str {
    MONTHS.get(month.wrapping_sub(1) as usize).copied().unwrap_or("")
}

/// `"ພະຈິກ 2026"`.
pub fn month_year_label(ym: YearMonth) -> String {
    format!("{} {}", month_name(ym.month), ym.year)
}

fn relative_prefix(date: Date, today: Date) -> Option<&'static str> {
    if date == today {
        Some("ມື້ນີ້")
    } else if date == today.prev_day() {
        Some("ມື້ວານ")
    } else {
        None
    }
}

/// Heading for a day in a list: `"ມື້ນີ້ · 11 ພະຈິກ"`, `"ມື້ວານ · 10 ພະຈິກ"`,
/// `"8 ພະຈິກ"`, or `"8 ພະຈິກ 2025"` when the year differs from today's.
pub fn day_label(date: Date, today: Date) -> String {
    let base = format!("{} {}", date.day, month_name(date.month));
    match relative_prefix(date, today) {
        Some(p) => format!("{p} · {base}"),
        None if date.year != today.year => format!("{base} {}", date.year),
        None => base,
    }
}

/// Date with year always shown, for the editor: `"ມື້ນີ້ · 11 ພະຈິກ 2026"`.
pub fn full_date_label(date: Date, today: Date) -> String {
    let base = format!("{} {} {}", date.day, month_name(date.month), date.year);
    match relative_prefix(date, today) {
        Some(p) => format!("{p} · {base}"),
        None => base,
    }
}

/// Keeps only digits from what the user typed. `"45,000"` -> `Some(45000)`.
pub fn parse_amount_input(s: &str) -> Option<i64> {
    let digits: String = s.chars().filter(char::is_ascii_digit).take(MAX_AMOUNT_DIGITS).collect();
    digits.parse().ok()
}

/// What to show in the amount box: `Some(45000)` -> `"45,000"`, `None` -> `""`.
pub fn format_amount_input(amount: Option<i64>) -> String {
    amount.map(format_number).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> Date {
        Date::new(y, m, day).unwrap()
    }

    #[test]
    fn format_number_groups_thousands() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(45000), "45,000");
        assert_eq!(format_number(1234567), "1,234,567");
        assert_eq!(format_number(-45000), "-45,000");
    }

    #[test]
    fn format_kip_appends_symbol() {
        assert_eq!(format_kip(45000), "45,000 ₭");
    }

    #[test]
    fn signed_kip_uses_kind_for_sign() {
        assert_eq!(signed_kip(Kind::Expense, 45000), "-45,000 ₭");
        assert_eq!(signed_kip(Kind::Income, 8_000_000), "+8,000,000 ₭");
    }

    #[test]
    fn net_kip_shows_sign_only_when_nonzero() {
        assert_eq!(net_kip(-145_000), "-145,000 ₭");
        assert_eq!(net_kip(1_500_000), "+1,500,000 ₭");
        assert_eq!(net_kip(0), "0 ₭");
    }

    #[test]
    fn month_names_are_lao() {
        assert_eq!(month_name(1), "ມັງກອນ");
        assert_eq!(month_name(11), "ພະຈິກ");
        assert_eq!(month_name(12), "ທັນວາ");
    }

    #[test]
    fn month_year_label_uses_gregorian_year() {
        assert_eq!(month_year_label(YearMonth { year: 2026, month: 11 }), "ພະຈິກ 2026");
    }

    #[test]
    fn day_label_names_today_and_yesterday() {
        let today = d(2026, 11, 11);
        assert_eq!(day_label(today, today), "ມື້ນີ້ · 11 ພະຈິກ");
        assert_eq!(day_label(d(2026, 11, 10), today), "ມື້ວານ · 10 ພະຈິກ");
        assert_eq!(day_label(d(2026, 11, 8), today), "8 ພະຈິກ");
        assert_eq!(day_label(d(2025, 11, 8), today), "8 ພະຈິກ 2025");
    }

    #[test]
    fn day_label_yesterday_across_new_year() {
        assert_eq!(day_label(d(2025, 12, 31), d(2026, 1, 1)), "ມື້ວານ · 31 ທັນວາ");
    }

    #[test]
    fn full_date_label_always_has_year() {
        let today = d(2026, 11, 11);
        assert_eq!(full_date_label(today, today), "ມື້ນີ້ · 11 ພະຈິກ 2026");
        assert_eq!(full_date_label(d(2026, 11, 8), today), "8 ພະຈິກ 2026");
    }

    #[test]
    fn parse_amount_input_keeps_digits_only() {
        assert_eq!(parse_amount_input("45,000"), Some(45000));
        assert_eq!(parse_amount_input(" 1 500 000 ₭"), Some(1_500_000));
        assert_eq!(parse_amount_input("0"), Some(0));
        assert_eq!(parse_amount_input(""), None);
        assert_eq!(parse_amount_input("abc"), None);
    }

    #[test]
    fn parse_amount_input_caps_length() {
        assert_eq!(parse_amount_input("12345678901234"), Some(123_456_789_012));
    }

    #[test]
    fn format_amount_input_round_trips() {
        assert_eq!(format_amount_input(Some(45000)), "45,000");
        assert_eq!(format_amount_input(None), "");
    }
}

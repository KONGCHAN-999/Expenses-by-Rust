//! Totals, category breakdowns, chart series and list grouping.

use crate::date::{Date, YearMonth};
use crate::model::{Kind, Transaction};

/// Remind the user to back up after this many days without one.
pub const BACKUP_REMINDER_DAYS: i64 = 14;
const MS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    Month(YearMonth),
    Year(i32),
}

impl Period {
    pub fn contains(&self, date: Date) -> bool {
        match *self {
            Period::Month(ym) => date.year_month() == ym,
            Period::Year(y) => date.year == y,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    pub income: i64,
    pub expense: i64,
}

impl Totals {
    pub fn balance(&self) -> i64 {
        self.income - self.expense
    }

    fn add(&mut self, tx: &Transaction) {
        match tx.kind {
            Kind::Income => self.income += tx.amount,
            Kind::Expense => self.expense += tx.amount,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Share {
    pub category_id: String,
    pub amount: i64,
    /// Whole percent; all shares of a breakdown add up to exactly 100.
    pub percent: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtherShare {
    pub amount: i64,
    pub percent: u32,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DayGroup {
    pub date: Date,
    pub items: Vec<Transaction>,
    /// Income minus expense for the day.
    pub net: i64,
}

pub fn totals(txs: &[Transaction], period: Period) -> Totals {
    let mut t = Totals::default();
    for tx in txs.iter().filter(|tx| period.contains(tx.date)) {
        t.add(tx);
    }
    t
}

/// Amount per category for one kind within a period, biggest first.
pub fn breakdown(txs: &[Transaction], kind: Kind, period: Period) -> Vec<Share> {
    let mut shares: Vec<Share> = Vec::new();
    for tx in txs.iter().filter(|t| t.kind == kind && period.contains(t.date)) {
        match shares.iter_mut().find(|s| s.category_id == tx.category_id) {
            Some(s) => s.amount += tx.amount,
            None => shares.push(Share { category_id: tx.category_id.clone(), amount: tx.amount, percent: 0 }),
        }
    }
    shares.sort_by(|a, b| b.amount.cmp(&a.amount).then_with(|| a.category_id.cmp(&b.category_id)));
    assign_percents(&mut shares);
    shares
}

/// Largest-remainder rounding so the whole percents always sum to 100.
fn assign_percents(shares: &mut [Share]) {
    let total: i128 = shares.iter().map(|s| s.amount as i128).sum();
    if total <= 0 {
        return;
    }
    let mut remainders: Vec<(usize, i128)> = Vec::with_capacity(shares.len());
    let mut assigned = 0u32;
    for (i, s) in shares.iter_mut().enumerate() {
        let scaled = s.amount as i128 * 100;
        s.percent = (scaled / total) as u32;
        assigned += s.percent;
        remainders.push((i, scaled % total));
    }
    // Stable sort keeps bigger shares first among equal remainders.
    remainders.sort_by(|a, b| b.1.cmp(&a.1));
    for (i, _) in remainders.into_iter().take(100u32.saturating_sub(assigned) as usize) {
        shares[i].percent += 1;
    }
}

/// Keeps the first `n` shares and folds the rest into one "other" entry.
/// A single leftover share is kept as-is rather than shown as "other (1)".
pub fn top_n(mut shares: Vec<Share>, n: usize) -> (Vec<Share>, Option<OtherShare>) {
    if shares.len() <= n + 1 {
        return (shares, None);
    }
    let rest = shares.split_off(n);
    let other = OtherShare {
        amount: rest.iter().map(|s| s.amount).sum(),
        percent: rest.iter().map(|s| s.percent).sum(),
        count: rest.len(),
    };
    (shares, Some(other))
}

/// Totals for `count` consecutive months ending at `end`, oldest first.
pub fn monthly_series(txs: &[Transaction], end: YearMonth, count: usize) -> Vec<(YearMonth, Totals)> {
    let mut months = Vec::with_capacity(count);
    let mut ym = end;
    for _ in 0..count {
        months.push(ym);
        ym = ym.prev();
    }
    months.reverse();
    months.into_iter().map(|ym| (ym, totals(txs, Period::Month(ym)))).collect()
}

/// Newest date first; same date -> most recently created first.
pub fn sort_newest_first(txs: &mut [Transaction]) {
    txs.sort_by(|a, b| {
        b.date
            .cmp(&a.date)
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| b.id.cmp(&a.id))
    });
}

/// Groups transactions (already sorted newest first) by date.
pub fn group_by_day(txs: Vec<Transaction>) -> Vec<DayGroup> {
    let mut groups: Vec<DayGroup> = Vec::new();
    for tx in txs {
        let signed = match tx.kind {
            Kind::Income => tx.amount,
            Kind::Expense => -tx.amount,
        };
        match groups.last_mut() {
            Some(g) if g.date == tx.date => {
                g.net += signed;
                g.items.push(tx);
            }
            _ => groups.push(DayGroup { date: tx.date, net: signed, items: vec![tx] }),
        }
    }
    groups
}

/// Case-insensitive search in the note and category name. Empty query matches all.
pub fn matches_query(tx: &Transaction, category_name: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || tx.note.to_lowercase().contains(&q) || category_name.to_lowercase().contains(&q)
}

/// Whole days between a backup and now (never negative).
pub fn days_since(then_ms: i64, now_ms: i64) -> i64 {
    (now_ms - then_ms).max(0) / MS_PER_DAY
}

pub fn needs_backup_reminder(tx_count: usize, last_backup_ms: Option<i64>, now_ms: i64) -> bool {
    tx_count > 0 && last_backup_ms.is_none_or(|last| days_since(last, now_ms) >= BACKUP_REMINDER_DAYS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tests::tx;

    const NOV: YearMonth = YearMonth { year: 2026, month: 11 };

    fn sample() -> Vec<Transaction> {
        vec![
            tx("1", Kind::Income, 8_000_000, "inc-salary", "2026-11-01"),
            tx("2", Kind::Expense, 45_000, "exp-food", "2026-11-11"),
            tx("3", Kind::Expense, 100_000, "exp-transport", "2026-11-11"),
            tx("4", Kind::Expense, 250_000, "exp-shopping", "2026-11-10"),
            tx("5", Kind::Expense, 999_000, "exp-food", "2026-10-31"),
            tx("6", Kind::Income, 1_500_000, "inc-side", "2025-11-08"),
        ]
    }

    #[test]
    fn period_month_and_year_contain_correct_dates() {
        let d = Date::parse_iso("2026-11-30").unwrap();
        assert!(Period::Month(NOV).contains(d));
        assert!(!Period::Month(NOV.prev()).contains(d));
        assert!(Period::Year(2026).contains(d));
        assert!(!Period::Year(2025).contains(d));
    }

    #[test]
    fn totals_only_count_the_period() {
        let t = totals(&sample(), Period::Month(NOV));
        assert_eq!(t, Totals { income: 8_000_000, expense: 395_000 });
        assert_eq!(t.balance(), 7_605_000);
    }

    #[test]
    fn totals_for_a_year() {
        let t = totals(&sample(), Period::Year(2026));
        assert_eq!(t.expense, 1_394_000);
        assert_eq!(t.income, 8_000_000);
    }

    #[test]
    fn breakdown_sums_per_category_biggest_first() {
        let shares = breakdown(&sample(), Kind::Expense, Period::Year(2026));
        let ids: Vec<_> = shares.iter().map(|s| (s.category_id.as_str(), s.amount)).collect();
        assert_eq!(
            ids,
            vec![("exp-food", 1_044_000), ("exp-shopping", 250_000), ("exp-transport", 100_000)]
        );
    }

    #[test]
    fn breakdown_percents_add_up_to_100_using_largest_remainder() {
        let txs = vec![
            tx("a", Kind::Expense, 1_560_000, "food", "2026-11-01"),
            tx("b", Kind::Expense, 1_300_000, "housing", "2026-11-01"),
            tx("c", Kind::Expense, 760_000, "transport", "2026-11-01"),
            tx("d", Kind::Expense, 700_000, "shopping", "2026-11-01"),
            tx("e", Kind::Expense, 1_100_000, "misc", "2026-11-01"),
        ];
        let shares = breakdown(&txs, Kind::Expense, Period::Month(NOV));
        let pct: Vec<_> = shares.iter().map(|s| (s.category_id.as_str(), s.percent)).collect();
        assert_eq!(
            pct,
            vec![("food", 29), ("housing", 24), ("misc", 20), ("transport", 14), ("shopping", 13)]
        );
    }

    #[test]
    fn breakdown_of_empty_period_is_empty() {
        assert!(breakdown(&sample(), Kind::Expense, Period::Year(2020)).is_empty());
    }

    fn share(id: &str, amount: i64, percent: u32) -> Share {
        Share { category_id: id.into(), amount, percent }
    }

    #[test]
    fn top_n_folds_the_rest_into_other() {
        let shares = vec![share("a", 50, 50), share("b", 30, 30), share("c", 15, 15), share("d", 5, 5)];
        let (top, other) = top_n(shares, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(other, Some(OtherShare { amount: 20, percent: 20, count: 2 }));
    }

    #[test]
    fn top_n_keeps_a_single_leftover() {
        let shares = vec![share("a", 50, 50), share("b", 30, 30), share("c", 20, 20)];
        let (top, other) = top_n(shares, 2);
        assert_eq!(top.len(), 3);
        assert_eq!(other, None);
    }

    #[test]
    fn monthly_series_is_oldest_first_and_includes_empty_months() {
        let series = monthly_series(&sample(), NOV, 3);
        let months: Vec<_> = series.iter().map(|(ym, _)| ym.month).collect();
        assert_eq!(months, vec![9, 10, 11]);
        assert_eq!(series[0].1, Totals::default());
        assert_eq!(series[1].1.expense, 999_000);
        assert_eq!(series[2].1.income, 8_000_000);
    }

    #[test]
    fn monthly_series_crosses_year_boundary() {
        let series = monthly_series(&[], YearMonth { year: 2026, month: 2 }, 3);
        let months: Vec<_> = series.iter().map(|(ym, _)| (ym.year, ym.month)).collect();
        assert_eq!(months, vec![(2025, 12), (2026, 1), (2026, 2)]);
    }

    #[test]
    fn sort_newest_first_uses_date_then_created_at() {
        let mut txs = sample();
        txs[1].created_at = 10; // id 2, 2026-11-11
        txs[2].created_at = 20; // id 3, 2026-11-11
        sort_newest_first(&mut txs);
        let ids: Vec<_> = txs.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["3", "2", "4", "1", "5", "6"]);
    }

    #[test]
    fn group_by_day_collects_same_dates_and_nets_them() {
        let mut txs = sample();
        sort_newest_first(&mut txs);
        let groups = group_by_day(txs);
        assert_eq!(groups.len(), 5);
        assert_eq!(groups[0].date, Date::parse_iso("2026-11-11").unwrap());
        assert_eq!(groups[0].items.len(), 2);
        assert_eq!(groups[0].net, -145_000);
        assert_eq!(groups[2].net, 8_000_000);
    }

    #[test]
    fn matches_query_searches_note_and_category_case_insensitively() {
        let mut t = tx("a", Kind::Expense, 1, "exp-food", "2026-11-01");
        t.note = "Coffee ກາເຟ".into();
        assert!(matches_query(&t, "ອາຫານ", ""));
        assert!(matches_query(&t, "ອາຫານ", "  coffee "));
        assert!(matches_query(&t, "ອາຫານ", "ກາເຟ"));
        assert!(matches_query(&t, "ອາຫານ", "ອາຫ"));
        assert!(!matches_query(&t, "ອາຫານ", "tea"));
    }

    #[test]
    fn days_since_counts_whole_days() {
        assert_eq!(days_since(0, MS_PER_DAY * 3 + 5), 3);
        assert_eq!(days_since(MS_PER_DAY, 0), 0);
    }

    #[test]
    fn backup_reminder_needs_data_and_an_old_or_missing_backup() {
        let now = MS_PER_DAY * 100;
        assert!(!needs_backup_reminder(0, None, now));
        assert!(needs_backup_reminder(1, None, now));
        assert!(!needs_backup_reminder(1, Some(now - MS_PER_DAY * 13), now));
        assert!(needs_backup_reminder(1, Some(now - MS_PER_DAY * 14), now));
    }
}

//! Backup (JSON) and spreadsheet (CSV) export/import, plus merging.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::date::Date;
use crate::model::{Category, Data, Kind, Transaction, other_category_id};

pub const BACKUP_APP_ID: &str = "lao-kip-expenses";
pub const BACKUP_VERSION: u32 = 1;
const BOM: char = '\u{FEFF}';
const CSV_HEADER: &str = "ວັນທີ,ປະເພດ,ໝວດ,ຈຳນວນ (ກີບ),ໝາຍເຫດ";
const EXPENSE_WORD: &str = "ລາຍຈ່າຍ";
const INCOME_WORD: &str = "ລາຍຮັບ";

#[derive(Serialize, Deserialize)]
struct Backup {
    app: String,
    version: u32,
    #[serde(default)]
    exported_at: String,
    categories: Vec<Category>,
    transactions: Vec<Transaction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowProblem {
    MissingFields,
    BadDate,
    BadKind,
    BadAmount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    NotABackup,
    NewerVersion(u32),
    NoRows,
    Row { line: usize, problem: RowProblem },
}

impl ImportError {
    /// Message shown to the user (Lao).
    pub fn message(&self) -> String {
        match self {
            ImportError::NotABackup => "ໄຟລ໌ນີ້ບໍ່ແມ່ນໄຟລ໌ສຳຮອງຂອງແອັບນີ້".into(),
            ImportError::NewerVersion(v) => {
                format!("ໄຟລ໌ສຳຮອງນີ້ມາຈາກແອັບເວີຊັນໃໝ່ກວ່າ (v{v}) ກະລຸນາອັບເດດແອັບກ່ອນ")
            }
            ImportError::NoRows => "ບໍ່ພົບລາຍການໃນໄຟລ໌ນີ້".into(),
            ImportError::Row { line, problem } => {
                let why = match problem {
                    RowProblem::MissingFields => "ຂໍ້ມູນບໍ່ຄົບ (ຕ້ອງມີ ວັນທີ, ປະເພດ, ໝວດ, ຈຳນວນ)",
                    RowProblem::BadDate => "ວັນທີບໍ່ຖືກຕ້ອງ (ໃຊ້ 2026-11-11 ຫຼື 11/11/2026)",
                    RowProblem::BadKind => "ປະເພດຕ້ອງເປັນ ລາຍຈ່າຍ ຫຼື ລາຍຮັບ",
                    RowProblem::BadAmount => "ຈຳນວນເງິນບໍ່ຖືກຕ້ອງ",
                };
                format!("ແຖວທີ {line}: {why}")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportSummary {
    pub transactions: usize,
    pub categories: usize,
    pub first: Option<Date>,
    pub last: Option<Date>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MergeReport {
    pub added: usize,
    pub skipped: usize,
    pub categories_added: usize,
}

#[derive(Serialize)]
struct BackupRef<'a> {
    app: &'a str,
    version: u32,
    exported_at: &'a str,
    categories: &'a [Category],
    transactions: &'a [Transaction],
}

pub fn export_json(data: &Data, exported_at: &str) -> String {
    let backup = BackupRef {
        app: BACKUP_APP_ID,
        version: BACKUP_VERSION,
        exported_at,
        categories: &data.categories,
        transactions: &data.transactions,
    };
    serde_json::to_string_pretty(&backup).expect("backup is always serializable")
}

pub fn import_json(text: &str) -> Result<Data, ImportError> {
    let value: serde_json::Value =
        serde_json::from_str(text.trim_start_matches(BOM)).map_err(|_| ImportError::NotABackup)?;
    if value.get("app").and_then(|v| v.as_str()) != Some(BACKUP_APP_ID) {
        return Err(ImportError::NotABackup);
    }
    let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    if version > BACKUP_VERSION {
        return Err(ImportError::NewerVersion(version));
    }
    let backup: Backup = serde_json::from_value(value).map_err(|_| ImportError::NotABackup)?;
    Ok(replace(Data { categories: backup.categories, transactions: backup.transactions }))
}

fn kind_word(kind: Kind) -> &'static str {
    match kind {
        Kind::Expense => EXPENSE_WORD,
        Kind::Income => INCOME_WORD,
    }
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// UTF-8 with BOM so Excel shows Lao text correctly. Oldest rows first.
pub fn export_csv(data: &Data) -> String {
    let mut txs: Vec<&Transaction> = data.transactions.iter().collect();
    txs.sort_by(|a, b| a.date.cmp(&b.date).then(a.created_at.cmp(&b.created_at)).then(a.id.cmp(&b.id)));
    let mut out = String::new();
    out.push(BOM);
    out.push_str(CSV_HEADER);
    out.push_str("\r\n");
    for t in txs {
        let category = data.category(&t.category_id).map(|c| c.name.as_str()).unwrap_or("");
        let row = [
            t.date.to_string(),
            kind_word(t.kind).to_string(),
            csv_field(category),
            t.amount.to_string(),
            csv_field(&t.note),
        ];
        out.push_str(&row.join(","));
        out.push_str("\r\n");
    }
    out
}

/// Splits CSV text into records, honouring quotes (including quoted newlines).
fn parse_csv_records(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' => in_quotes = true,
            ',' => record.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}

fn parse_date(s: &str) -> Option<Date> {
    Date::parse_iso(s).or_else(|| Date::parse_dmy(s))
}

fn parse_kind(s: &str) -> Option<Kind> {
    let s = s.trim().to_lowercase();
    if s == EXPENSE_WORD || s == "expense" {
        Some(Kind::Expense)
    } else if s == INCOME_WORD || s == "income" {
        Some(Kind::Income)
    } else {
        None
    }
}

/// Accepts `45000`, `45,000`, `45,000 ₭`, `45000.00`. Sign is ignored.
fn parse_csv_amount(s: &str) -> Option<i64> {
    let cleaned: String = s.chars().filter(|c| !matches!(c, ',' | ' ' | '₭' | '-' | '+')).collect();
    let (whole, frac) = cleaned.split_once('.').unwrap_or((&cleaned, ""));
    let digits_only = |p: &str| p.chars().all(|c| c.is_ascii_digit());
    if whole.is_empty() || whole.len() > 15 || !digits_only(whole) || !digits_only(frac) {
        return None;
    }
    whole.parse().ok()
}

fn norm(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Reads rows `date, type, category, amount[, note]`. Categories are matched by
/// name against `existing`; unknown names become new categories.
pub fn import_csv(
    text: &str,
    existing: &Data,
    new_id: &mut dyn FnMut() -> String,
    now_ms: i64,
) -> Result<Data, ImportError> {
    let mut out = Data { categories: existing.categories.clone(), transactions: Vec::new() };
    out.ensure_other_categories();
    let mut by_name: HashMap<(Kind, String), String> =
        out.categories.iter().map(|c| ((c.kind, norm(&c.name)), c.id.clone())).collect();
    let mut seen_first = false;

    for (idx, rec) in parse_csv_records(text.trim_start_matches(BOM)).iter().enumerate() {
        let line = idx + 1;
        let err = |problem| ImportError::Row { line, problem };
        if rec.iter().all(|f| f.trim().is_empty()) {
            continue;
        }
        let is_first = !seen_first;
        seen_first = true;
        if is_first && parse_date(&rec[0]).is_none() && parse_kind(rec.get(1).map_or("", |s| s)).is_none() {
            continue; // header row
        }
        if rec.len() < 4 {
            return Err(err(RowProblem::MissingFields));
        }
        let date = parse_date(&rec[0]).ok_or(err(RowProblem::BadDate))?;
        let kind = parse_kind(&rec[1]).ok_or(err(RowProblem::BadKind))?;
        let amount = parse_csv_amount(&rec[3]).ok_or(err(RowProblem::BadAmount))?;
        let name = rec[2].trim();
        let category_id = if name.is_empty() {
            other_category_id(kind).to_string()
        } else if let Some(id) = by_name.get(&(kind, norm(name))) {
            id.clone()
        } else {
            let cat = Category {
                id: new_id(),
                kind,
                name: name.to_string(),
                icon: "🏷️".into(),
                color: "#64748b".into(),
            };
            by_name.insert((kind, norm(name)), cat.id.clone());
            let id = cat.id.clone();
            out.upsert_category(cat);
            id
        };
        out.transactions.push(Transaction {
            id: new_id(),
            kind,
            amount,
            category_id,
            date,
            note: rec.get(4).map_or("", |s| s.trim()).to_string(),
            created_at: now_ms + idx as i64,
        });
    }

    if out.transactions.is_empty() {
        return Err(ImportError::NoRows);
    }
    Ok(out)
}

/// Picks JSON or CSV from the file name, falling back to sniffing the content.
pub fn import_any(
    file_name: &str,
    text: &str,
    existing: &Data,
    new_id: &mut dyn FnMut() -> String,
    now_ms: i64,
) -> Result<Data, ImportError> {
    let name = file_name.to_lowercase();
    let looks_json = text.trim_start_matches(BOM).trim_start().starts_with('{');
    if name.ends_with(".json") || (!name.ends_with(".csv") && looks_json) {
        import_json(text)
    } else {
        import_csv(text, existing, new_id, now_ms)
    }
}

pub fn summarize(data: &Data) -> ImportSummary {
    ImportSummary {
        transactions: data.transactions.len(),
        categories: data.categories.len(),
        first: data.transactions.iter().map(|t| t.date).min(),
        last: data.transactions.iter().map(|t| t.date).max(),
    }
}

type ContentKey = (Date, Kind, i64, String, String);

fn content_key(t: &Transaction) -> ContentKey {
    (t.date, t.kind, t.amount, t.category_id.clone(), t.note.trim().to_string())
}

/// Adds incoming transactions that aren't already present. A transaction is
/// "already present" if its id exists, or if an identical one (same date, kind,
/// amount, category and note) exists that hasn't been matched yet.
pub fn merge(existing: &Data, incoming: Data) -> (Data, MergeReport) {
    let mut out = existing.clone();
    out.ensure_other_categories();
    let mut report = MergeReport::default();

    // Map incoming category ids onto existing ones (by id, then by name).
    let mut cat_map: HashMap<String, String> = HashMap::new();
    for cat in incoming.categories {
        let target = if out.category(&cat.id).is_some() {
            cat.id.clone()
        } else if let Some(found) =
            out.categories.iter().find(|c| c.kind == cat.kind && norm(&c.name) == norm(&cat.name))
        {
            found.id.clone()
        } else {
            report.categories_added += 1;
            let id = cat.id.clone();
            out.upsert_category(cat.clone());
            id
        };
        cat_map.insert(cat.id, target);
    }

    let mut unmatched: HashMap<ContentKey, usize> = HashMap::new();
    for t in &existing.transactions {
        *unmatched.entry(content_key(t)).or_default() += 1;
    }
    let (known, fresh): (Vec<_>, Vec<_>) = incoming
        .transactions
        .into_iter()
        .partition(|t| existing.transactions.iter().any(|e| e.id == t.id));

    for t in known {
        report.skipped += 1;
        if let Some(e) = existing.transactions.iter().find(|e| e.id == t.id) {
            if let Some(n) = unmatched.get_mut(&content_key(e)) {
                *n = n.saturating_sub(1);
            }
        }
    }
    for mut t in fresh {
        t.amount = t.amount.abs();
        t.category_id = cat_map.get(&t.category_id).cloned().unwrap_or(t.category_id);
        if out.category(&t.category_id).is_none_or(|c| c.kind != t.kind) {
            t.category_id = other_category_id(t.kind).to_string();
        }
        match unmatched.get_mut(&content_key(&t)) {
            Some(n) if *n > 0 => {
                *n -= 1;
                report.skipped += 1;
            }
            _ => {
                out.transactions.push(t);
                report.added += 1;
            }
        }
    }
    (out, report)
}

/// Uses the incoming data as-is (after making sure it is self-consistent).
pub fn replace(incoming: Data) -> Data {
    let mut d = incoming;
    d.ensure_other_categories();
    let kinds: HashMap<String, Kind> = d.categories.iter().map(|c| (c.id.clone(), c.kind)).collect();
    for t in &mut d.transactions {
        t.amount = t.amount.abs();
        if kinds.get(&t.category_id) != Some(&t.kind) {
            t.category_id = other_category_id(t.kind).to_string();
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tests::tx;
    use crate::model::{OTHER_EXPENSE_ID, OTHER_INCOME_ID};

    fn ids() -> impl FnMut() -> String {
        let mut n = 0;
        move || {
            n += 1;
            format!("gen-{n}")
        }
    }

    fn sample() -> Data {
        let mut data = Data::new_default();
        let mut t1 = tx("t1", Kind::Expense, 45_000, "exp-food", "2026-11-11");
        t1.note = "ເຝີ, ກາເຟ \"ດຳ\"".into();
        data.upsert_transaction(t1);
        data.upsert_transaction(tx("t2", Kind::Income, 8_000_000, "inc-salary", "2026-11-01"));
        data
    }

    fn content(d: &Data) -> Vec<(String, Kind, i64, String, String)> {
        let mut v: Vec<_> = d
            .transactions
            .iter()
            .map(|t| {
                let cat = d.category(&t.category_id).map(|c| c.name.clone()).unwrap_or_default();
                (t.date.to_string(), t.kind, t.amount, cat, t.note.clone())
            })
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
        v
    }

    // ---------- JSON ----------

    #[test]
    fn json_round_trips() {
        let data = sample();
        let text = export_json(&data, "2026-11-11T09:00:00Z");
        assert!(text.contains(BACKUP_APP_ID));
        assert_eq!(import_json(&text).unwrap(), data);
    }

    #[test]
    fn json_rejects_garbage_and_other_apps() {
        assert_eq!(import_json("hello"), Err(ImportError::NotABackup));
        let other = r#"{"app":"x","version":1,"categories":[],"transactions":[]}"#;
        assert_eq!(import_json(other), Err(ImportError::NotABackup));
    }

    #[test]
    fn json_rejects_newer_versions() {
        let newer = format!(r#"{{"app":"{BACKUP_APP_ID}","version":99,"categories":[],"transactions":[]}}"#);
        assert_eq!(import_json(&newer), Err(ImportError::NewerVersion(99)));
    }

    #[test]
    fn json_import_repairs_unknown_categories_and_missing_fallbacks() {
        let text = format!(
            r#"{{"app":"{BACKUP_APP_ID}","version":1,"categories":[],
               "transactions":[{{"id":"a","kind":"income","amount":5,"category_id":"gone","date":"2026-01-02"}}]}}"#
        );
        let data = import_json(&text).unwrap();
        assert_eq!(data.transactions[0].category_id, OTHER_INCOME_ID);
        assert!(data.category(OTHER_EXPENSE_ID).is_some());
    }

    // ---------- CSV export ----------

    #[test]
    fn csv_starts_with_bom_and_lao_header() {
        let csv = export_csv(&sample());
        assert!(csv.starts_with('\u{FEFF}'));
        assert_eq!(csv.trim_start_matches('\u{FEFF}').lines().next().unwrap(), CSV_HEADER);
    }

    #[test]
    fn csv_rows_are_oldest_first_and_quote_when_needed() {
        let csv = export_csv(&sample());
        let rows: Vec<_> = csv.split("\r\n").filter(|l| !l.is_empty()).skip(1).collect();
        assert_eq!(rows[0], "2026-11-01,ລາຍຮັບ,ເງິນເດືອນ,8000000,");
        assert_eq!(rows[1], "2026-11-11,ລາຍຈ່າຍ,ອາຫານ,45000,\"ເຝີ, ກາເຟ \"\"ດຳ\"\"\"");
    }

    // ---------- CSV import ----------

    #[test]
    fn csv_round_trips_content_and_reuses_existing_categories() {
        let data = sample();
        let csv = export_csv(&data);
        let back = import_csv(&csv, &Data::new_default(), &mut ids(), 1000).unwrap();
        assert_eq!(content(&back), content(&data));
        let food = back.transactions.iter().find(|t| t.amount == 45_000).unwrap();
        assert_eq!(food.category_id, "exp-food");
        assert_eq!(food.id, "gen-2", "ids come from the generator");
    }

    #[test]
    fn csv_accepts_english_words_dmy_dates_grouped_amounts_and_no_note() {
        let csv = "date,type,category,amount\n11/11/2026,Expense,ອາຫານ,\"45,000\"\n";
        let d = import_csv(csv, &Data::new_default(), &mut ids(), 0).unwrap();
        assert_eq!(d.transactions.len(), 1);
        let t = &d.transactions[0];
        assert_eq!((t.kind, t.amount, t.date.to_string()), (Kind::Expense, 45_000, "2026-11-11".to_string()));
        assert_eq!(t.note, "");
    }

    #[test]
    fn csv_creates_categories_for_unknown_names_once() {
        let csv = "2026-11-01,ລາຍຈ່າຍ,ລ້ຽງໝາ,10000\n2026-11-02,ລາຍຈ່າຍ,ລ້ຽງໝາ,20000\n";
        let d = import_csv(csv, &Data::new_default(), &mut ids(), 0).unwrap();
        let pets: Vec<_> = d.categories.iter().filter(|c| c.name == "ລ້ຽງໝາ").collect();
        assert_eq!(pets.len(), 1);
        assert_eq!(pets[0].kind, Kind::Expense);
        assert!(d.transactions.iter().all(|t| t.category_id == pets[0].id));
    }

    #[test]
    fn csv_blank_category_goes_to_other() {
        let csv = "2026-11-01,ລາຍຮັບ,,10000\n";
        let d = import_csv(csv, &Data::new_default(), &mut ids(), 0).unwrap();
        assert_eq!(d.transactions[0].category_id, OTHER_INCOME_ID);
    }

    #[test]
    fn csv_keeps_newlines_inside_quotes() {
        let csv = "2026-11-01,ລາຍຈ່າຍ,ອາຫານ,1000,\"line1\nline2\"\n";
        let d = import_csv(csv, &Data::new_default(), &mut ids(), 0).unwrap();
        assert_eq!(d.transactions[0].note, "line1\nline2");
    }

    #[test]
    fn csv_reports_the_failing_line() {
        let cases = [
            ("2026-11-01,ລາຍຈ່າຍ,ອາຫານ,1000\n2026-13-01,ລາຍຈ່າຍ,ອາຫານ,1000\n", 2, RowProblem::BadDate),
            ("2026-11-01,ໂອນ,ອາຫານ,1000\n", 1, RowProblem::BadKind),
            ("2026-11-01,ລາຍຈ່າຍ,ອາຫານ,abc\n", 1, RowProblem::BadAmount),
            ("2026-11-01,ລາຍຈ່າຍ\n", 1, RowProblem::MissingFields),
        ];
        for (csv, line, problem) in cases {
            assert_eq!(
                import_csv(csv, &Data::new_default(), &mut ids(), 0),
                Err(ImportError::Row { line, problem }),
                "{csv}"
            );
        }
    }

    #[test]
    fn csv_line_numbers_count_the_header() {
        let csv = format!("{CSV_HEADER}\n2026-11-01,ລາຍຈ່າຍ,ອາຫານ,x\n");
        assert_eq!(
            import_csv(&csv, &Data::new_default(), &mut ids(), 0),
            Err(ImportError::Row { line: 2, problem: RowProblem::BadAmount })
        );
    }

    #[test]
    fn csv_with_only_a_header_has_no_rows() {
        let csv = format!("\u{FEFF}{CSV_HEADER}\r\n\r\n");
        assert_eq!(import_csv(&csv, &Data::new_default(), &mut ids(), 0), Err(ImportError::NoRows));
    }

    #[test]
    fn import_any_detects_format() {
        let data = sample();
        let json = export_json(&data, "");
        let csv = export_csv(&data);
        let existing = Data::new_default();
        assert_eq!(import_any("backup.json", &json, &existing, &mut ids(), 0).unwrap(), data);
        assert_eq!(import_any("whatever.txt", &json, &existing, &mut ids(), 0).unwrap(), data);
        let from_csv = import_any("export.CSV", &csv, &existing, &mut ids(), 0).unwrap();
        assert_eq!(content(&from_csv), content(&data));
    }

    #[test]
    fn error_messages_are_lao_and_include_line() {
        let msg = ImportError::Row { line: 7, problem: RowProblem::BadDate }.message();
        assert!(msg.contains('7') && msg.contains("ວັນທີ"), "{msg}");
        assert!(!ImportError::NotABackup.message().is_empty());
    }

    // ---------- summary / merge / replace ----------

    #[test]
    fn summarize_counts_and_finds_date_range() {
        let s = summarize(&sample());
        assert_eq!(s.transactions, 2);
        assert_eq!(s.categories, 18);
        assert_eq!(s.first.unwrap().to_string(), "2026-11-01");
        assert_eq!(s.last.unwrap().to_string(), "2026-11-11");
        assert_eq!(summarize(&Data::default()).first, None);
    }

    #[test]
    fn merge_skips_known_ids_and_adds_new_ones() {
        let existing = sample();
        let mut incoming = sample();
        incoming.upsert_transaction(tx("t3", Kind::Expense, 20_000, "exp-food", "2026-11-12"));
        let (merged, report) = merge(&existing, incoming);
        assert_eq!(report, MergeReport { added: 1, skipped: 2, categories_added: 0 });
        assert_eq!(merged.transactions.len(), 3);
    }

    #[test]
    fn merge_matches_categories_by_name_and_skips_identical_content() {
        let existing = sample();
        // Same data re-imported from CSV gets fresh ids.
        let incoming = import_csv(&export_csv(&existing), &existing, &mut ids(), 0).unwrap();
        let (merged, report) = merge(&existing, incoming);
        assert_eq!(report.added, 0);
        assert_eq!(report.skipped, 2);
        assert_eq!(merged, existing);
    }

    #[test]
    fn merge_keeps_extra_identical_rows_beyond_existing_ones() {
        let mut existing = Data::new_default();
        existing.upsert_transaction(tx("a", Kind::Expense, 20_000, "exp-food", "2026-11-12"));
        let mut incoming = Data::new_default();
        incoming.upsert_transaction(tx("x", Kind::Expense, 20_000, "exp-food", "2026-11-12"));
        incoming.upsert_transaction(tx("y", Kind::Expense, 20_000, "exp-food", "2026-11-12"));
        let (merged, report) = merge(&existing, incoming);
        assert_eq!((report.added, report.skipped), (1, 1));
        assert_eq!(merged.transactions.len(), 2);
    }

    #[test]
    fn merge_adds_new_categories_and_remaps_by_name() {
        let existing = Data::new_default();
        let mut incoming = Data::new_default();
        // Same name as an existing category but a different id.
        incoming.categories.push(Category {
            id: "food-2".into(),
            kind: Kind::Expense,
            name: "ອາຫານ".into(),
            icon: "🍲".into(),
            color: "#000".into(),
        });
        incoming.categories.push(Category {
            id: "pets".into(),
            kind: Kind::Expense,
            name: "ລ້ຽງໝາ".into(),
            icon: "🐶".into(),
            color: "#000".into(),
        });
        incoming.upsert_transaction(tx("x", Kind::Expense, 1, "food-2", "2026-11-12"));
        incoming.upsert_transaction(tx("y", Kind::Expense, 2, "pets", "2026-11-12"));
        let (merged, report) = merge(&existing, incoming);
        assert_eq!(report.categories_added, 1);
        assert_eq!(merged.category("pets").unwrap().name, "ລ້ຽງໝາ");
        assert_eq!(merged.transactions.iter().find(|t| t.id == "x").unwrap().category_id, "exp-food");
    }

    #[test]
    fn replace_restores_fallback_categories() {
        let incoming = Data { categories: vec![], transactions: vec![] };
        let d = replace(incoming);
        assert!(d.category(OTHER_EXPENSE_ID).is_some());
        assert!(d.category(OTHER_INCOME_ID).is_some());
    }
}

//! Data model: categories, transactions and the whole app dataset.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::date::Date;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Expense,
    Income,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub icon: String,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub kind: Kind,
    /// Whole kip, always positive. The sign comes from `kind`.
    pub amount: i64,
    pub category_id: String,
    pub date: Date,
    #[serde(default)]
    pub note: String,
    /// Unix milliseconds; breaks ties between transactions on the same date.
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub categories: Vec<Category>,
    pub transactions: Vec<Transaction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CategoryError {
    NotFound,
    /// The fallback "other" category of each kind cannot be deleted.
    Protected,
}

pub const OTHER_EXPENSE_ID: &str = "exp-other";
pub const OTHER_INCOME_ID: &str = "inc-other";
pub const SNACK_ID: &str = "exp-snack";

/// Bumped whenever a default category is added, so data saved by an older
/// version can receive it once (see [`Data::add_new_defaults`]).
/// 1: first release. 2: snack (ຂະໜົມ).
pub const DEFAULTS_VERSION: u32 = 2;

pub fn other_category_id(kind: Kind) -> &'static str {
    match kind {
        Kind::Expense => OTHER_EXPENSE_ID,
        Kind::Income => OTHER_INCOME_ID,
    }
}

fn cat(id: &str, kind: Kind, name: &str, icon: &str, color: &str) -> Category {
    Category { id: id.into(), kind, name: name.into(), icon: icon.into(), color: color.into() }
}

fn other_category(kind: Kind) -> Category {
    cat(other_category_id(kind), kind, "ອື່ນໆ", "📦", "#94a3b8")
}

fn snack_category() -> Category {
    cat(SNACK_ID, Kind::Expense, "ຂະໜົມ", "🍩", "#d946ef")
}

pub fn default_categories() -> Vec<Category> {
    use Kind::*;
    vec![
        cat("exp-food", Expense, "ອາຫານ", "🍜", "#f59e0b"),
        snack_category(),
        cat("exp-transport", Expense, "ເດີນທາງ", "🚗", "#0ea5e9"),
        cat("exp-household", Expense, "ເຄື່ອງໃຊ້", "🛒", "#84cc16"),
        cat("exp-housing", Expense, "ທີ່ພັກ", "🏠", "#6366f1"),
        cat("exp-utilities", Expense, "ຄ່ານ້ຳໄຟ", "💡", "#eab308"),
        cat("exp-phone", Expense, "ໂທລະສັບ", "📱", "#14b8a6"),
        cat("exp-fun", Expense, "ບັນເທີງ", "🎮", "#a855f7"),
        cat("exp-health", Expense, "ສຸຂະພາບ", "💊", "#ef4444"),
        cat("exp-shopping", Expense, "ຊ໋ອບປິ້ງ", "🛍️", "#ec4899"),
        cat("exp-education", Expense, "ການສຶກສາ", "📚", "#3b82f6"),
        cat("exp-merit", Expense, "ທຳບຸນ", "🙏", "#f97316"),
        other_category(Expense),
        cat("inc-salary", Income, "ເງິນເດືອນ", "💰", "#16a34a"),
        cat("inc-side", Income, "ວຽກເສີມ", "💼", "#0d9488"),
        cat("inc-gift", Income, "ຂອງຂວັນ", "🎁", "#db2777"),
        cat("inc-invest", Income, "ລົງທຶນ", "📈", "#2563eb"),
        other_category(Income),
    ]
}

impl Data {
    pub fn new_default() -> Data {
        Data { categories: default_categories(), transactions: Vec::new() }
    }

    pub fn category(&self, id: &str) -> Option<&Category> {
        self.categories.iter().find(|c| c.id == id)
    }

    pub fn categories_of(&self, kind: Kind) -> Vec<&Category> {
        self.categories.iter().filter(|c| c.kind == kind).collect()
    }

    pub fn upsert_transaction(&mut self, tx: Transaction) {
        match self.transactions.iter_mut().find(|t| t.id == tx.id) {
            Some(existing) => *existing = tx,
            None => self.transactions.push(tx),
        }
    }

    pub fn delete_transaction(&mut self, id: &str) -> bool {
        let before = self.transactions.len();
        self.transactions.retain(|t| t.id != id);
        self.transactions.len() != before
    }

    pub fn upsert_category(&mut self, cat: Category) {
        if let Some(existing) = self.categories.iter_mut().find(|c| c.id == cat.id) {
            *existing = cat;
            return;
        }
        // Keep the "other" fallback last within its kind.
        let other = other_category_id(cat.kind);
        match self.categories.iter().position(|c| c.id == other) {
            Some(pos) => self.categories.insert(pos, cat),
            None => self.categories.push(cat),
        }
    }

    /// Deletes a category and moves its transactions to the "other" category
    /// of the same kind. Returns how many transactions were moved.
    pub fn delete_category(&mut self, id: &str) -> Result<usize, CategoryError> {
        if id == OTHER_EXPENSE_ID || id == OTHER_INCOME_ID {
            return Err(CategoryError::Protected);
        }
        let kind = self.category(id).ok_or(CategoryError::NotFound)?.kind;
        self.ensure_other_categories();
        let other = other_category_id(kind);
        let mut moved = 0;
        for t in self.transactions.iter_mut().filter(|t| t.category_id == id) {
            t.category_id = other.to_string();
            moved += 1;
        }
        self.categories.retain(|c| c.id != id);
        Ok(moved)
    }

    pub fn transaction_count_in(&self, category_id: &str) -> usize {
        self.transactions.iter().filter(|t| t.category_id == category_id).count()
    }

    /// Adds the default categories introduced after defaults version `seen`
    /// (see [`DEFAULTS_VERSION`]), unless one with the same id or name exists.
    pub fn add_new_defaults(&mut self, seen: u32) {
        if seen < 2 {
            self.insert_default(snack_category(), "exp-food");
        }
    }

    fn insert_default(&mut self, cat: Category, after_id: &str) {
        let taken = self
            .categories
            .iter()
            .any(|c| c.id == cat.id || (c.kind == cat.kind && c.name.trim() == cat.name));
        if taken {
            return;
        }
        match self.categories.iter().position(|c| c.id == after_id) {
            Some(pos) => self.categories.insert(pos + 1, cat),
            None => self.upsert_category(cat),
        }
    }

    /// Re-adds the "other" categories if they are missing (e.g. after an import).
    pub fn ensure_other_categories(&mut self) {
        for kind in [Kind::Expense, Kind::Income] {
            if self.category(other_category_id(kind)).is_none() {
                self.categories.push(other_category(kind));
            }
        }
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Date, D::Error> {
        let s = String::deserialize(d)?;
        Date::parse_iso(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid date: {s}")))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn tx(id: &str, kind: Kind, amount: i64, cat: &str, date: &str) -> Transaction {
        Transaction {
            id: id.into(),
            kind,
            amount,
            category_id: cat.into(),
            date: Date::parse_iso(date).unwrap(),
            note: String::new(),
            created_at: 0,
        }
    }

    #[test]
    fn date_serializes_as_iso_string() {
        let t = tx("a", Kind::Expense, 45000, "exp-food", "2026-11-01");
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains(r#""date":"2026-11-01""#), "{json}");
        assert!(json.contains(r#""kind":"expense""#), "{json}");
        let back: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn invalid_date_string_fails_to_deserialize() {
        let json = r#"{"id":"a","kind":"income","amount":1,"category_id":"x","date":"2026-02-30"}"#;
        assert!(serde_json::from_str::<Transaction>(json).is_err());
    }

    #[test]
    fn missing_note_and_created_at_default() {
        let json = r#"{"id":"a","kind":"income","amount":1,"category_id":"x","date":"2026-02-03"}"#;
        let t: Transaction = serde_json::from_str(json).unwrap();
        assert_eq!(t.note, "");
        assert_eq!(t.created_at, 0);
    }

    #[test]
    fn default_categories_have_13_expense_and_5_income_with_unique_ids() {
        let cats = default_categories();
        assert_eq!(cats.iter().filter(|c| c.kind == Kind::Expense).count(), 13);
        assert_eq!(cats.iter().filter(|c| c.kind == Kind::Income).count(), 5);
        let mut ids: Vec<_> = cats.iter().map(|c| c.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 18);
        assert!(ids.contains(&OTHER_EXPENSE_ID));
        assert!(ids.contains(&OTHER_INCOME_ID));
    }

    #[test]
    fn other_category_id_matches_kind() {
        assert_eq!(other_category_id(Kind::Expense), OTHER_EXPENSE_ID);
        assert_eq!(other_category_id(Kind::Income), OTHER_INCOME_ID);
    }

    #[test]
    fn categories_of_filters_by_kind_and_keeps_order() {
        let data = Data::new_default();
        let income: Vec<_> = data.categories_of(Kind::Income).iter().map(|c| c.id.clone()).collect();
        assert_eq!(income.len(), 5);
        assert_eq!(income.last().unwrap(), OTHER_INCOME_ID);
    }

    #[test]
    fn upsert_transaction_inserts_then_replaces() {
        let mut data = Data::new_default();
        data.upsert_transaction(tx("a", Kind::Expense, 100, "exp-food", "2026-11-01"));
        data.upsert_transaction(tx("a", Kind::Expense, 250, "exp-food", "2026-11-01"));
        assert_eq!(data.transactions.len(), 1);
        assert_eq!(data.transactions[0].amount, 250);
    }

    #[test]
    fn delete_transaction_reports_whether_it_existed() {
        let mut data = Data::new_default();
        data.upsert_transaction(tx("a", Kind::Expense, 100, "exp-food", "2026-11-01"));
        assert!(data.delete_transaction("a"));
        assert!(!data.delete_transaction("a"));
        assert!(data.transactions.is_empty());
    }

    #[test]
    fn upsert_category_replaces_by_id_and_appends_new() {
        let mut data = Data::new_default();
        let mut food = data.category("exp-food").unwrap().clone();
        food.name = "ກິນ".into();
        data.upsert_category(food);
        assert_eq!(data.category("exp-food").unwrap().name, "ກິນ");
        let before = data.categories.len();
        data.upsert_category(Category {
            id: "new".into(),
            kind: Kind::Expense,
            name: "ລົດ".into(),
            icon: "🛵".into(),
            color: "#000000".into(),
        });
        assert_eq!(data.categories.len(), before + 1);
    }

    #[test]
    fn new_expense_category_is_inserted_before_other() {
        let mut data = Data::new_default();
        data.upsert_category(Category {
            id: "new".into(),
            kind: Kind::Expense,
            name: "ລົດ".into(),
            icon: "🛵".into(),
            color: "#000000".into(),
        });
        let ids: Vec<_> = data.categories_of(Kind::Expense).iter().map(|c| c.id.clone()).collect();
        assert_eq!(ids[ids.len() - 2], "new");
        assert_eq!(ids[ids.len() - 1], OTHER_EXPENSE_ID);
    }

    #[test]
    fn delete_category_moves_its_transactions_to_other() {
        let mut data = Data::new_default();
        data.upsert_transaction(tx("a", Kind::Expense, 100, "exp-food", "2026-11-01"));
        data.upsert_transaction(tx("b", Kind::Expense, 100, "exp-food", "2026-11-02"));
        data.upsert_transaction(tx("c", Kind::Expense, 100, "exp-transport", "2026-11-02"));
        assert_eq!(data.delete_category("exp-food"), Ok(2));
        assert!(data.category("exp-food").is_none());
        assert_eq!(data.transaction_count_in(OTHER_EXPENSE_ID), 2);
        assert_eq!(data.transaction_count_in("exp-transport"), 1);
    }

    #[test]
    fn delete_category_refuses_other_and_unknown() {
        let mut data = Data::new_default();
        assert_eq!(data.delete_category(OTHER_INCOME_ID), Err(CategoryError::Protected));
        assert_eq!(data.delete_category("nope"), Err(CategoryError::NotFound));
    }

    #[test]
    fn ensure_other_categories_restores_missing_fallbacks() {
        let mut data = Data { categories: vec![], transactions: vec![] };
        data.ensure_other_categories();
        assert!(data.category(OTHER_EXPENSE_ID).is_some());
        assert!(data.category(OTHER_INCOME_ID).is_some());
        data.ensure_other_categories();
        assert_eq!(data.categories.len(), 2);
    }

    /// Data saved before the snack category existed (defaults version 1).
    fn data_from_version_1() -> Data {
        let mut data = Data::new_default();
        data.categories.retain(|c| c.id != SNACK_ID);
        data
    }

    #[test]
    fn snack_is_a_default_expense_category_right_after_food() {
        let data = Data::new_default();
        let ids: Vec<_> = data.categories_of(Kind::Expense).iter().map(|c| c.id.as_str()).collect();
        assert_eq!(&ids[..2], &["exp-food", SNACK_ID]);
        assert_eq!(data.category(SNACK_ID).unwrap().name, "ຂະໜົມ");
    }

    #[test]
    fn add_new_defaults_brings_snack_to_old_data() {
        let mut data = data_from_version_1();
        data.add_new_defaults(1);
        let ids: Vec<_> = data.categories_of(Kind::Expense).iter().map(|c| c.id.as_str()).collect();
        assert_eq!(&ids[..2], &["exp-food", SNACK_ID]);
    }

    #[test]
    fn add_new_defaults_puts_snack_before_other_when_food_is_gone() {
        let mut data = data_from_version_1();
        data.delete_category("exp-food").unwrap();
        data.add_new_defaults(1);
        let ids: Vec<_> = data.categories_of(Kind::Expense).iter().map(|c| c.id.as_str()).collect();
        assert_eq!(&ids[ids.len() - 2..], &[SNACK_ID, OTHER_EXPENSE_ID]);
    }

    #[test]
    fn add_new_defaults_skips_what_the_user_already_has_or_removed() {
        // Same name made by hand: no duplicate.
        let mut data = data_from_version_1();
        data.upsert_category(cat("mine", Kind::Expense, " ຂະໜົມ ", "🍰", "#ec4899"));
        data.add_new_defaults(1);
        assert!(data.category(SNACK_ID).is_none());

        // Deleted after it was added: stays deleted.
        let mut data = Data::new_default();
        data.delete_category(SNACK_ID).unwrap();
        data.add_new_defaults(DEFAULTS_VERSION);
        assert!(data.category(SNACK_ID).is_none());

        // Running twice adds it once.
        let mut data = data_from_version_1();
        data.add_new_defaults(1);
        data.add_new_defaults(1);
        assert_eq!(data.categories.iter().filter(|c| c.id == SNACK_ID).count(), 1);
    }
}

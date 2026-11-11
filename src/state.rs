//! App-wide reactive state, persisted to localStorage.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use expenses_core::date::{Date, YearMonth};
use expenses_core::model::{DEFAULTS_VERSION, Data, Transaction};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::platform;

const DATA_KEY: &str = "lao-kip-expenses/data/v1";
const SETTINGS_KEY: &str = "lao-kip-expenses/settings/v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    List,
    Report,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Editor {
    New,
    Edit(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub last_backup_ms: Option<i64>,
    /// Which default categories this device's data has already been given
    /// (see `expenses_core::model::DEFAULTS_VERSION`).
    #[serde(default)]
    pub defaults_version: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub message: String,
    /// A just-deleted transaction the user can bring back.
    pub undo: Option<Transaction>,
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub data: RwSignal<Data>,
    pub settings: RwSignal<Settings>,
    pub tab: RwSignal<Tab>,
    /// Month shown on the home and summary screens.
    pub month: RwSignal<YearMonth>,
    pub editor: RwSignal<Option<Editor>>,
    pub toast: RwSignal<Option<Toast>>,
    pub today: RwSignal<Date>,
}

static TOAST_IDS: AtomicU64 = AtomicU64::new(1);

impl AppState {
    pub fn load() -> AppState {
        let today = platform::today();
        let mut settings: Settings = platform::load(SETTINGS_KEY)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let mut data = load_data();
        if settings.defaults_version < DEFAULTS_VERSION {
            // Give older data the categories added since (once; deleting them later sticks).
            data.add_new_defaults(settings.defaults_version);
            settings.defaults_version = DEFAULTS_VERSION;
        }
        AppState {
            data: RwSignal::new(data),
            settings: RwSignal::new(settings),
            tab: RwSignal::new(Tab::Home),
            month: RwSignal::new(today.year_month()),
            editor: RwSignal::new(None),
            toast: RwSignal::new(None),
            today: RwSignal::new(today),
        }
    }

    /// Saves data and settings to localStorage whenever they change.
    pub fn persist(self) {
        Effect::new(move |_| {
            let json = self.data.with(|d| serde_json::to_string(d).unwrap_or_default());
            if platform::store(DATA_KEY, &json).is_err() {
                platform::alert("ບັນທຶກຂໍ້ມູນບໍ່ສຳເລັດ: ພື້ນທີ່ໃນເຄື່ອງອາດເຕັມ");
            }
        });
        Effect::new(move |_| {
            let json = self.settings.with(|s| serde_json::to_string(s).unwrap_or_default());
            let _ = platform::store(SETTINGS_KEY, &json);
        });
    }

    pub fn notify(self, message: impl Into<String>) {
        self.show_toast(message.into(), None);
    }

    pub fn notify_with_undo(self, message: impl Into<String>, deleted: Transaction) {
        self.show_toast(message.into(), Some(deleted));
    }

    fn show_toast(self, message: String, undo: Option<Transaction>) {
        let id = TOAST_IDS.fetch_add(1, Ordering::Relaxed);
        let wait = if undo.is_some() { 4500 } else { 2200 };
        self.toast.set(Some(Toast { id, message, undo }));
        let toast = self.toast;
        set_timeout(
            move || {
                if toast.with_untracked(|t| t.as_ref().map(|t| t.id)) == Some(id) {
                    toast.set(None);
                }
            },
            Duration::from_millis(wait),
        );
    }

    /// The app can stay open past midnight on a phone.
    pub fn refresh_today(self) {
        let today = platform::today();
        if self.today.get_untracked() != today {
            self.today.set(today);
        }
    }

    pub fn open_new_entry(self) {
        self.refresh_today();
        self.editor.set(Some(Editor::New));
    }
}

fn load_data() -> Data {
    let Some(raw) = platform::load(DATA_KEY) else {
        return Data::new_default();
    };
    match serde_json::from_str::<Data>(&raw) {
        Ok(mut data) => {
            data.ensure_other_categories();
            data
        }
        Err(_) => {
            // Never overwrite data we couldn't read: keep a copy next to it.
            let _ = platform::store(&format!("{DATA_KEY}/unreadable-{}", platform::now_ms()), &raw);
            platform::alert("ອ່ານຂໍ້ມູນເກົ່າບໍ່ໄດ້ ໄດ້ເກັບສຳເນົາໄວ້ແລ້ວ ແລະເລີ່ມຂໍ້ມູນໃໝ່");
            Data::new_default()
        }
    }
}

pub fn use_app() -> AppState {
    expect_context::<AppState>()
}

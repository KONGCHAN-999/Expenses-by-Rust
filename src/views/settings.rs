use expenses_core::format::month_year_label;
use expenses_core::io::{self, ImportSummary};
use expenses_core::model::{Data, Kind};
use expenses_core::summary::{days_since, needs_backup_reminder};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::platform::{self, SaveOutcome};
use crate::state::{Theme, use_app};
use crate::views::categories::CategoriesView;

/// "ສຳຮອງຫຼ້າສຸດ 3 ມື້ກ່ອນ" etc.
pub fn backup_status_text(last_backup_ms: Option<i64>) -> String {
    match last_backup_ms {
        None => "ຍັງບໍ່ເຄີຍສຳຮອງຂໍ້ມູນ".into(),
        Some(ms) => match days_since(ms, platform::now_ms()) {
            0 => "ສຳຮອງຫຼ້າສຸດ ມື້ນີ້".into(),
            d => format!("ສຳຮອງຫຼ້າສຸດ {d} ມື້ກ່ອນ"),
        },
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Main,
    Categories(Kind),
}

#[derive(Clone, PartialEq)]
struct PendingImport {
    file_name: String,
    summary: ImportSummary,
    data: Data,
}

#[component]
pub fn SettingsView() -> impl IntoView {
    let page = RwSignal::new(Page::Main);
    move || match page.get() {
        Page::Main => view! { <SettingsMain page=page/> }.into_any(),
        Page::Categories(kind) => {
            view! { <CategoriesView kind=kind on_back=Callback::new(move |_| page.set(Page::Main))/> }.into_any()
        }
    }
}

#[component]
fn SettingsMain(page: RwSignal<Page>) -> impl IntoView {
    let state = use_app();
    let pending = RwSignal::new(None::<PendingImport>);

    let export_json = move |_| {
        let content = state.data.with_untracked(|d| io::export_json(d, &platform::iso_timestamp()));
        let name = format!("kip-backup-{}.json", state.today.get_untracked());
        let promise = platform::start_save(&name, "application/json", &content);
        spawn_local(async move {
            match platform::finish_save(promise).await {
                SaveOutcome::Saved => {
                    state.settings.update(|s| s.last_backup_ms = Some(platform::now_ms()));
                    state.notify("ສົ່ງອອກໄຟລ໌ສຳຮອງແລ້ວ");
                }
                SaveOutcome::Cancelled => {}
                SaveOutcome::Failed(e) => platform::alert(&format!("ສົ່ງອອກບໍ່ສຳເລັດ: {e}")),
            }
        });
    };
    let export_csv = move |_| {
        let content = state.data.with_untracked(io::export_csv);
        let name = format!("kip-export-{}.csv", state.today.get_untracked());
        let promise = platform::start_save(&name, "text/csv", &content);
        spawn_local(async move {
            match platform::finish_save(promise).await {
                SaveOutcome::Saved => state.notify("ສົ່ງອອກ CSV ແລ້ວ"),
                SaveOutcome::Cancelled => {}
                SaveOutcome::Failed(e) => platform::alert(&format!("ສົ່ງອອກບໍ່ສຳເລັດ: {e}")),
            }
        });
    };
    let pick_file = move |ev: web_sys::Event| {
        let input: web_sys::HtmlInputElement = event_target(&ev);
        let Some(file) = input.files().and_then(|f| f.get(0)) else { return };
        input.set_value(""); // allow choosing the same file again later
        let file_name = file.name();
        spawn_local(async move {
            let text = match platform::read_file_text(file).await {
                Ok(t) => t,
                Err(_) => return platform::alert("ອ່ານໄຟລ໌ບໍ່ໄດ້"),
            };
            let existing = state.data.get_untracked();
            let mut new_id = platform::new_id;
            match io::import_any(&file_name, &text, &existing, &mut new_id, platform::now_ms()) {
                Ok(data) => pending.set(Some(PendingImport { file_name, summary: io::summarize(&data), data })),
                Err(e) => platform::alert(&e.message()),
            }
        });
    };

    let backup_needed = move || {
        let count = state.data.with(|d| d.transactions.len());
        needs_backup_reminder(count, state.settings.with(|s| s.last_backup_ms), platform::now_ms())
    };
    let count_of = move |k: Kind| state.data.with(|d| d.categories_of(k).len());
    let theme_value = move || match state.settings.with(|s| s.theme) {
        Theme::System => "system",
        Theme::Light => "light",
        Theme::Dark => "dark",
    };
    let set_theme = move |ev| {
        let theme = match event_target_value(&ev).as_str() {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::System,
        };
        state.settings.update(|s| s.theme = theme);
    };
    let delete_all = move |_| {
        if platform::confirm("ລຶບຂໍ້ມູນທັງໝົດບໍ? ການກະທຳນີ້ຍົກເລີກບໍ່ໄດ້ ແນະນຳໃຫ້ສົ່ງອອກໄຟລ໌ສຳຮອງກ່ອນ") {
            state.data.set(Data::new_default());
            state.notify("ລຶບຂໍ້ມູນທັງໝົດແລ້ວ");
        }
    };

    view! {
        <div class="page">
            <h1 class="page-title">"ຕັ້ງຄ່າ"</h1>
            <div class="note" class:warn=backup_needed>
                "ຂໍ້ມູນເກັບໄວ້ໃນເຄື່ອງນີ້ເທົ່ານັ້ນ ຄວນສຳຮອງເປັນປະຈຳ · "
                {move || backup_status_text(state.settings.with(|s| s.last_backup_ms))}
            </div>

            <div class="group-label">"ຂໍ້ມູນ"</div>
            <div class="group">
                <button class="row" on:click=export_json>
                    <span class="row-icon" style="background:#5b5bd6">"⬆"</span>
                    <span class="row-text">"ສົ່ງອອກໄຟລ໌ສຳຮອງ"<small>".json · ນຳເຂົ້າຄືນໄດ້ຄົບທຸກຢ່າງ"</small></span>
                    <span class="chev">"›"</span>
                </button>
                <button class="row" on:click=export_csv>
                    <span class="row-icon" style="background:#1a9e5a">"▦"</span>
                    <span class="row-text">"ສົ່ງອອກເປັນ CSV"<small>"ເປີດດ້ວຍ Excel / Google Sheets"</small></span>
                    <span class="chev">"›"</span>
                </button>
                <label class="row">
                    <span class="row-icon" style="background:#0ea5e9">"⬇"</span>
                    <span class="row-text">"ນຳເຂົ້າຂໍ້ມູນ"<small>"ຈາກໄຟລ໌ .json ຫຼື .csv"</small></span>
                    <span class="chev">"›"</span>
                    <input type="file" class="visually-hidden" on:change=pick_file/>
                </label>
            </div>

            <div class="group-label">"ໝວດໝູ່"</div>
            <div class="group">
                <button class="row" on:click=move |_| page.set(Page::Categories(Kind::Expense))>
                    <span class="row-icon" style="background:#e5484d">"−"</span>
                    <span class="row-text">"ໝວດລາຍຈ່າຍ"</span>
                    <span class="row-meta">{move || count_of(Kind::Expense)}</span>
                    <span class="chev">"›"</span>
                </button>
                <button class="row" on:click=move |_| page.set(Page::Categories(Kind::Income))>
                    <span class="row-icon" style="background:#1a9e5a">"+"</span>
                    <span class="row-text">"ໝວດລາຍຮັບ"</span>
                    <span class="row-meta">{move || count_of(Kind::Income)}</span>
                    <span class="chev">"›"</span>
                </button>
            </div>

            <div class="group-label">"ອື່ນໆ"</div>
            <div class="group">
                <label class="row">
                    <span class="row-icon" style="background:#8e8e93">"◐"</span>
                    <span class="row-text">"ຮູບແບບສີ"</span>
                    <select class="row-select" prop:value=theme_value on:change=set_theme>
                        <option value="system">"ຕາມລະບົບ"</option>
                        <option value="light">"ສະຫວ່າງ"</option>
                        <option value="dark">"ມືດ"</option>
                    </select>
                </label>
                <button class="row danger-row" on:click=delete_all>"ລຶບຂໍ້ມູນທັງໝົດ"</button>
            </div>

            <p class="footer">
                {move || format!("ລາຍການທັງໝົດ {} · ເວີຊັນ {}", state.data.with(|d| d.transactions.len()), env!("CARGO_PKG_VERSION"))}
            </p>

            {move || pending.get().map(|p| view! { <ImportSheet pending=p on_close=Callback::new(move |_| pending.set(None))/> })}
        </div>
    }
}

fn range_label(summary: &ImportSummary) -> String {
    match (summary.first, summary.last) {
        (Some(a), Some(b)) if a.year_month() == b.year_month() => month_year_label(a.year_month()),
        (Some(a), Some(b)) => format!("{} – {}", month_year_label(a.year_month()), month_year_label(b.year_month())),
        _ => String::new(),
    }
}

#[component]
fn ImportSheet(pending: PendingImport, on_close: Callback<()>) -> impl IntoView {
    let state = use_app();
    let PendingImport { file_name, summary, data } = pending;
    let incoming = StoredValue::new(data);
    let range = range_label(&summary);
    let details = if range.is_empty() {
        format!("ພົບ {} ລາຍການ", summary.transactions)
    } else {
        format!("ພົບ {} ລາຍການ · {range}", summary.transactions)
    };

    let merge = move |_| {
        let (merged, report) = state.data.with_untracked(|d| io::merge(d, incoming.get_value()));
        state.data.set(merged);
        on_close.run(());
        state.notify(if report.skipped > 0 {
            format!("ນຳເຂົ້າ {} ລາຍການ · ຂ້າມ {} ລາຍການທີ່ຊ້ຳ", report.added, report.skipped)
        } else {
            format!("ນຳເຂົ້າ {} ລາຍການແລ້ວ", report.added)
        });
    };
    let replace = move |_| {
        if !platform::confirm("ຂໍ້ມູນເກົ່າທັງໝົດໃນເຄື່ອງຈະຖືກລຶບ ແລະແທນທີ່ດ້ວຍຂໍ້ມູນຈາກໄຟລ໌ ແນ່ໃຈບໍ?") {
            return;
        }
        let data = io::replace(incoming.get_value());
        let count = data.transactions.len();
        state.data.set(data);
        on_close.run(());
        state.notify(format!("ແທນທີ່ຂໍ້ມູນແລ້ວ ({count} ລາຍການ)"));
    };

    view! {
        <div class="backdrop" on:click=move |_| on_close.run(())></div>
        <div class="sheet" role="dialog" aria-label="ນຳເຂົ້າຂໍ້ມູນ">
            <h3>"ນຳເຂົ້າຂໍ້ມູນ"</h3>
            <p class="sheet-text">"ໄຟລ໌ " <b>{file_name}</b> <br/> {details}</p>
            <button class="sheet-btn primary" on:click=merge>
                "ລວມກັບຂໍ້ມູນເກົ່າ"
                <small>"ລາຍການທີ່ຊ້ຳກັນຈະຖືກຂ້າມ"</small>
            </button>
            <button class="sheet-btn danger" on:click=replace>
                "ແທນທີ່ຂໍ້ມູນທັງໝົດ"
                <small>"ຂໍ້ມູນເກົ່າໃນເຄື່ອງຈະຖືກລຶບ"</small>
            </button>
            <button class="sheet-btn plain" on:click=move |_| on_close.run(())>"ຍົກເລີກ"</button>
        </div>
    }
}

use expenses_core::format::{format_kip, month_year_label};
use expenses_core::model::{Kind, Transaction};
use expenses_core::summary::{Period, breakdown, needs_backup_reminder, sort_newest_first, totals};
use leptos::prelude::*;

use crate::components::{CatIcon, Empty, PeriodNav, TxRow};
use crate::platform;
use crate::state::{Tab, use_app};
use crate::views::settings::backup_status_text;

const RECENT_COUNT: usize = 5;
const TOP_COUNT: usize = 3;

#[component]
pub fn HomeView() -> impl IntoView {
    let state = use_app();
    let period = move || Period::Month(state.month.get());

    let month_totals = Memo::new(move |_| state.data.with(|d| totals(&d.transactions, period())));
    let top = Memo::new(move |_| {
        state.data.with(|d| {
            breakdown(&d.transactions, Kind::Expense, period())
                .into_iter()
                .take(TOP_COUNT)
                .filter_map(|s| d.category(&s.category_id).cloned().map(|c| (c, s.amount)))
                .collect::<Vec<_>>()
        })
    });
    let recent = Memo::new(move |_| {
        let ym = state.month.get();
        state.data.with(|d| {
            let mut v: Vec<Transaction> = d.transactions.iter().filter(|t| t.date.year_month() == ym).cloned().collect();
            sort_newest_first(&mut v);
            v.truncate(RECENT_COUNT);
            v
        })
    });
    let show_backup_banner = move || {
        let count = state.data.with(|d| d.transactions.len());
        needs_backup_reminder(count, state.settings.with(|s| s.last_backup_ms), platform::now_ms())
    };
    let balance_label = move || {
        if state.month.get() == state.today.get().year_month() { "ຍອດເຫຼືອເດືອນນີ້" } else { "ຍອດເຫຼືອ" }
    };

    view! {
        <div class="page">
            <PeriodNav
                label=Signal::derive(move || month_year_label(state.month.get()))
                on_prev=Callback::new(move |_| state.month.update(|m| *m = m.prev()))
                on_next=Callback::new(move |_| state.month.update(|m| *m = m.next()))
            />

            <Show when=show_backup_banner>
                <button class="banner" on:click=move |_| state.tab.set(Tab::Settings)>
                    "⚠️ " {move || backup_status_text(state.settings.with(|s| s.last_backup_ms))} " · ແຕະເພື່ອສຳຮອງ"
                </button>
            </Show>

            <section class="balance">
                <div class="balance-label">{balance_label}</div>
                <div class="balance-value">{move || format_kip(month_totals.get().balance())}</div>
                <div class="balance-split">
                    <div>
                        <span>"↑ ລາຍຮັບ"</span>
                        <b>{move || format_kip(month_totals.get().income)}</b>
                    </div>
                    <div>
                        <span>"↓ ລາຍຈ່າຍ"</span>
                        <b>{move || format_kip(month_totals.get().expense)}</b>
                    </div>
                </div>
            </section>

            <Show when=move || !top.with(Vec::is_empty)>
                <section class="card">
                    <div class="card-head">
                        <span>"ໃຊ້ຈ່າຍຫຼາຍທີ່ສຸດ"</span>
                        <button class="link" on:click=move |_| state.tab.set(Tab::Report)>"ເບິ່ງສະຫຼຸບ ›"</button>
                    </div>
                    {move || {
                        let items = top.get();
                        let max = items.first().map(|(_, a)| *a).unwrap_or(1).max(1);
                        items
                            .into_iter()
                            .map(|(c, amount)| {
                                let width = format!("width:{}%;background:{}", amount * 100 / max, c.color);
                                view! {
                                    <div class="catbar">
                                        <CatIcon icon=c.icon color=c.color/>
                                        <div class="catbar-main">
                                            <div class="catbar-top">
                                                <span>{c.name}</span>
                                                <span>{format_kip(amount)}</span>
                                            </div>
                                            <div class="track"><i style=width></i></div>
                                        </div>
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                </section>
            </Show>

            <section class="card">
                <div class="card-head">
                    <span>"ລາຍການຫຼ້າສຸດ"</span>
                    <button class="link" on:click=move |_| state.tab.set(Tab::List)>"ທັງໝົດ ›"</button>
                </div>
                <Show
                    when=move || !recent.with(Vec::is_empty)
                    fallback=|| view! { <Empty icon="🧾" title="ເດືອນນີ້ຍັງບໍ່ມີລາຍການ" hint="ກົດປຸ່ມ + ເພື່ອເພີ່ມລາຍການ"/> }
                >
                    <div class="tx-list">
                        {move || recent.get().into_iter().map(|tx| view! { <TxRow tx=tx show_day=true/> }).collect_view()}
                    </div>
                </Show>
            </section>
        </div>
    }
}

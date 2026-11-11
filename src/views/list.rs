use expenses_core::format::{day_label, net_kip};
use expenses_core::model::{Kind, Transaction};
use expenses_core::summary::{group_by_day, matches_query, sort_newest_first};
use leptos::prelude::*;

use crate::components::{Empty, TxRow};
use crate::state::use_app;

const PAGE_SIZE: usize = 150;

#[component]
pub fn ListView() -> impl IntoView {
    let state = use_app();
    let query = RwSignal::new(String::new());
    let kind_filter = RwSignal::new(None::<Kind>);
    let category_filter = RwSignal::new(String::new()); // "" = all categories
    let limit = RwSignal::new(PAGE_SIZE);

    let filtered = Memo::new(move |_| {
        let q = query.get();
        let kind = kind_filter.get();
        let cat = category_filter.get();
        state.data.with(|d| {
            let mut txs: Vec<Transaction> = d
                .transactions
                .iter()
                .filter(|t| kind.is_none_or(|k| t.kind == k))
                .filter(|t| cat.is_empty() || t.category_id == cat)
                .filter(|t| {
                    let name = d.category(&t.category_id).map_or("", |c| c.name.as_str());
                    matches_query(t, name, &q)
                })
                .cloned()
                .collect();
            sort_newest_first(&mut txs);
            txs
        })
    });
    let groups = Memo::new(move |_| {
        let mut txs = filtered.get();
        txs.truncate(limit.get());
        group_by_day(txs)
    });
    let has_more = move || filtered.with(Vec::len) > limit.get();

    let set_kind = move |k: Option<Kind>| {
        kind_filter.set(k);
        limit.set(PAGE_SIZE);
        let cat = category_filter.get_untracked();
        let still_valid = cat.is_empty()
            || state.data.with_untracked(|d| d.category(&cat).is_some_and(|c| k.is_none_or(|k| c.kind == k)));
        if !still_valid {
            category_filter.set(String::new());
        }
    };
    let chip = move |label: &'static str, k: Option<Kind>| {
        view! {
            <button class="chip" class:on=move || kind_filter.get() == k on:click=move |_| set_kind(k)>
                {label}
            </button>
        }
    };
    let category_options = move || {
        let k = kind_filter.get();
        state.data.with(|d| {
            d.categories
                .iter()
                .filter(|c| k.is_none_or(|k| c.kind == k))
                .map(|c| {
                    let label = format!("{} {}", c.icon, c.name);
                    view! { <option value=c.id.clone()>{label}</option> }
                })
                .collect_view()
        })
    };
    let nothing_at_all = move || state.data.with(|d| d.transactions.is_empty());

    view! {
        <div class="page">
            <h1 class="page-title">"ລາຍການ"</h1>
            <input
                class="search"
                type="search"
                placeholder="🔍 ຄົ້ນຫາໝາຍເຫດ ຫຼື ໝວດ…"
                prop:value=move || query.get()
                on:input=move |ev| {
                    query.set(event_target_value(&ev));
                    limit.set(PAGE_SIZE);
                }
            />
            <div class="chips">
                {chip("ທັງໝົດ", None)}
                {chip("ລາຍຈ່າຍ", Some(Kind::Expense))}
                {chip("ລາຍຮັບ", Some(Kind::Income))}
                <select
                    class="chip chip-select"
                    class:on=move || !category_filter.get().is_empty()
                    prop:value=move || category_filter.get()
                    on:change=move |ev| {
                        category_filter.set(event_target_value(&ev));
                        limit.set(PAGE_SIZE);
                    }
                >
                    <option value="">"ໝວດ ▾"</option>
                    {category_options}
                </select>
            </div>

            <Show
                when=move || !groups.with(Vec::is_empty)
                fallback=move || {
                    if nothing_at_all() {
                        view! { <Empty icon="🧾" title="ຍັງບໍ່ມີລາຍການ" hint="ກົດປຸ່ມ + ເພື່ອເພີ່ມລາຍການທຳອິດ"/> }.into_any()
                    } else {
                        view! { <Empty icon="🔍" title="ບໍ່ພົບລາຍການທີ່ກົງກັນ"/> }.into_any()
                    }
                }
            >
                {move || {
                    let today = state.today.get();
                    groups
                        .get()
                        .into_iter()
                        .map(|g| {
                            let net_class = if g.net < 0 { "neg" } else { "pos" };
                            view! {
                                <div class="day-head">
                                    <span>{day_label(g.date, today)}</span>
                                    <span class=net_class>{net_kip(g.net)}</span>
                                </div>
                                <div class="card tx-list">
                                    {g.items.into_iter().map(|tx| view! { <TxRow tx=tx/> }).collect_view()}
                                </div>
                            }
                        })
                        .collect_view()
                }}
                <Show when=has_more>
                    <button class="more-btn" on:click=move |_| limit.update(|l| *l += PAGE_SIZE)>"ສະແດງເພີ່ມ"</button>
                </Show>
                <p class="hint">"ແຕະເພື່ອແກ້ໄຂ · ປັດໄປທາງຊ້າຍເພື່ອລຶບ"</p>
            </Show>
        </div>
    }
}

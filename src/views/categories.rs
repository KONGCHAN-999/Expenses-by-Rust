//! Add, rename, recolor and delete categories.

use expenses_core::model::{Category, Kind, OTHER_EXPENSE_ID, OTHER_INCOME_ID};
use leptos::prelude::*;

use crate::components::{CatIcon, icon_background};
use crate::platform;
use crate::state::use_app;

const ICONS: &[&str] = &[
    "🍜", "🍚", "🍩", "🥤", "☕", "🍺", "🚗", "🛵", "⛽", "🚌", "✈️", "🛒", "🏠", "💡", "💧", "📱", "💻",
    "🎮", "🎬", "🎵", "💊", "🏥", "👕", "👟", "🛍️", "💄", "📚", "🎓", "🙏", "🎁", "👶", "🐶", "🔧",
    "🧾", "💳", "🏦", "💰", "💼", "📈", "🏆", "🎉", "🏷️", "📦",
];
const COLORS: &[&str] = &[
    "#f59e0b", "#f97316", "#ef4444", "#ec4899", "#d946ef", "#a855f7", "#6366f1", "#3b82f6", "#0ea5e9", "#14b8a6",
    "#16a34a", "#84cc16", "#eab308", "#64748b", "#94a3b8",
];

#[component]
pub fn CategoriesView(kind: Kind, on_back: Callback<()>) -> impl IntoView {
    let state = use_app();
    // Some(None) = adding a new one, Some(Some(cat)) = editing.
    let editing = RwSignal::new(None::<Option<Category>>);
    let title = match kind {
        Kind::Expense => "ໝວດລາຍຈ່າຍ",
        Kind::Income => "ໝວດລາຍຮັບ",
    };

    let rows = move || {
        state.data.with(|d| {
            d.categories_of(kind)
                .into_iter()
                .map(|c| {
                    let count = d.transaction_count_in(&c.id);
                    let cat = c.clone();
                    view! {
                        <button class="row" on:click=move |_| editing.set(Some(Some(cat.clone())))>
                            <CatIcon icon=c.icon.clone() color=c.color.clone()/>
                            <span class="row-text">{c.name.clone()}<small>{format!("{count} ລາຍການ")}</small></span>
                            <span class="chev">"›"</span>
                        </button>
                    }
                })
                .collect_view()
        })
    };

    view! {
        <div class="page">
            <button class="back" on:click=move |_| on_back.run(())>"‹ ຕັ້ງຄ່າ"</button>
            <h1 class="page-title">{title}</h1>
            <div class="group">{rows}</div>
            <button class="add-btn" on:click=move |_| editing.set(Some(None))>"+ ເພີ່ມໝວດ"</button>
            <p class="hint">"ຖ້າລຶບໝວດ ລາຍການໃນໝວດນັ້ນຈະຍ້າຍໄປ \"ອື່ນໆ\""</p>
            {move || {
                editing
                    .get()
                    .map(|target| {
                        view! { <CategorySheet kind=kind target=target on_close=Callback::new(move |_| editing.set(None))/> }
                    })
            }}
        </div>
    }
}

#[component]
fn CategorySheet(kind: Kind, target: Option<Category>, on_close: Callback<()>) -> impl IntoView {
    let state = use_app();
    let is_new = target.is_none();
    let id = target.as_ref().map(|c| c.id.clone());
    let protected = id.as_deref().is_some_and(|i| i == OTHER_EXPENSE_ID || i == OTHER_INCOME_ID);
    let name = RwSignal::new(target.as_ref().map(|c| c.name.clone()).unwrap_or_default());
    let icon = RwSignal::new(target.as_ref().map_or_else(|| "🏷️".to_string(), |c| c.icon.clone()));
    let color = RwSignal::new(target.as_ref().map_or_else(|| COLORS[0].to_string(), |c| c.color.clone()));
    let id = StoredValue::new(id);

    let save = move |_| {
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() {
            return state.notify("ກະລຸນາໃສ່ຊື່ໝວດ");
        }
        let current_id = id.get_value();
        let duplicate = state.data.with_untracked(|d| {
            d.categories_of(kind).iter().any(|c| {
                c.name.trim().to_lowercase() == new_name.to_lowercase() && Some(&c.id) != current_id.as_ref()
            })
        });
        if duplicate {
            return state.notify("ມີໝວດຊື່ນີ້ແລ້ວ");
        }
        let cat = Category {
            id: current_id.unwrap_or_else(platform::new_id),
            kind,
            name: new_name,
            icon: icon.get_untracked(),
            color: color.get_untracked(),
        };
        state.data.update(|d| d.upsert_category(cat));
        on_close.run(());
        state.notify("ບັນທຶກໝວດແລ້ວ");
    };
    let delete = move |_| {
        let Some(cat_id) = id.get_value() else { return };
        let count = state.data.with_untracked(|d| d.transaction_count_in(&cat_id));
        let question = if count > 0 {
            format!("ລຶບໝວດ \"{}\" ບໍ? ລາຍການ {count} ລາຍການຈະຍ້າຍໄປ \"ອື່ນໆ\"", name.get_untracked())
        } else {
            format!("ລຶບໝວດ \"{}\" ບໍ?", name.get_untracked())
        };
        if !platform::confirm(&question) {
            return;
        }
        let mut result = Ok(0);
        state.data.update(|d| result = d.delete_category(&cat_id));
        on_close.run(());
        if result.is_ok() {
            state.notify("ລຶບໝວດແລ້ວ");
        }
    };

    view! {
        <div class="backdrop" on:click=move |_| on_close.run(())></div>
        <div class="sheet" role="dialog">
            <h3>{if is_new { "ເພີ່ມໝວດ" } else { "ແກ້ໄຂໝວດ" }}</h3>
            <div class="cat-preview">
                <span class="cat-icon big" style=move || icon_background(&color.get())>{move || icon.get()}</span>
                <input
                    class="text-input"
                    placeholder="ຊື່ໝວດ"
                    maxlength="30"
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                />
            </div>
            <div class="emoji-grid">
                {ICONS
                    .iter()
                    .map(|e| {
                        let e = *e;
                        view! { <button class:on=move || icon.get() == e on:click=move |_| icon.set(e.to_string())>{e}</button> }
                    })
                    .collect_view()}
            </div>
            <div class="color-row">
                {COLORS
                    .iter()
                    .map(|c| {
                        let c = *c;
                        view! {
                            <button
                                class="swatch"
                                class:on=move || color.get() == c
                                style=format!("background:{c}")
                                aria-label=c
                                on:click=move |_| color.set(c.to_string())
                            ></button>
                        }
                    })
                    .collect_view()}
            </div>
            <button class="sheet-btn primary" on:click=save>"ບັນທຶກ"</button>
            {(!is_new && !protected).then(|| view! { <button class="sheet-btn danger" on:click=delete>"ລຶບໝວດນີ້"</button> })}
            <button class="sheet-btn plain" on:click=move |_| on_close.run(())>"ຍົກເລີກ"</button>
        </div>
    }
}

//! Full-screen form to add or edit a transaction.

use expenses_core::date::Date;
use expenses_core::format::{format_amount_input, full_date_label, parse_amount_input};
use expenses_core::model::{Kind, Transaction, other_category_id};
use leptos::prelude::*;

use crate::components::icon_background;
use crate::platform;
use crate::state::{Editor, use_app};

#[component]
pub fn EditorView() -> impl IntoView {
    let state = use_app();
    let existing = match state.editor.get_untracked() {
        Some(Editor::Edit(id)) => state.data.with_untracked(|d| d.transactions.iter().find(|t| t.id == id).cloned()),
        _ => None,
    };
    let is_edit = existing.is_some();
    let first_category = move |k: Kind| {
        state.data.with_untracked(|d| {
            d.categories_of(k).first().map(|c| c.id.clone()).unwrap_or_else(|| other_category_id(k).to_string())
        })
    };

    let kind = RwSignal::new(existing.as_ref().map_or(Kind::Expense, |t| t.kind));
    let amount = RwSignal::new(existing.as_ref().map(|t| t.amount));
    let category = RwSignal::new(existing.as_ref().map_or_else(|| first_category(Kind::Expense), |t| t.category_id.clone()));
    let date = RwSignal::new(existing.as_ref().map_or_else(|| state.today.get_untracked(), |t| t.date));
    let note = RwSignal::new(existing.as_ref().map(|t| t.note.clone()).unwrap_or_default());
    let original = StoredValue::new(existing.map(|t| (t.id, t.created_at)));

    let set_kind = move |k: Kind| {
        if kind.get_untracked() != k {
            kind.set(k);
            category.set(first_category(k));
        }
    };
    let close = move || state.editor.set(None);
    let save = Callback::new(move |_: ()| {
        let Some(value) = amount.get_untracked().filter(|v| *v > 0) else {
            state.notify("ກະລຸນາໃສ່ຈຳນວນເງິນ");
            return;
        };
        let (id, created_at) = original.get_value().unwrap_or_else(|| (platform::new_id(), platform::now_ms()));
        let tx = Transaction {
            id,
            kind: kind.get_untracked(),
            amount: value,
            category_id: category.get_untracked(),
            date: date.get_untracked(),
            note: note.get_untracked().trim().to_string(),
            created_at,
        };
        state.data.update(|d| d.upsert_transaction(tx));
        close();
        state.notify("ບັນທຶກແລ້ວ");
    });
    let delete = move |_| {
        let Some((id, _)) = original.get_value() else { return };
        if !platform::confirm("ລຶບລາຍການນີ້ບໍ?") {
            return;
        }
        let removed = state.data.with_untracked(|d| d.transactions.iter().find(|t| t.id == id).cloned());
        state.data.update(|d| {
            d.delete_transaction(&id);
        });
        close();
        if let Some(tx) = removed {
            state.notify_with_undo("ລຶບລາຍການແລ້ວ", tx);
        }
    };
    let save_on_enter = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            ev.prevent_default();
            save.run(());
        }
    };

    let category_grid = move || {
        let k = kind.get();
        let cats: Vec<_> = state.data.with(|d| d.categories_of(k).into_iter().cloned().collect());
        cats.into_iter()
            .map(|c| {
                let id = c.id.clone();
                let selected = Memo::new(move |_| category.with(|cur| *cur == id));
                let id = c.id.clone();
                view! {
                    <button
                        class="cat-choice"
                        class:on=move || selected.get()
                        style=move || if selected.get() { icon_background(&c.color) } else { String::new() }
                        on:click=move |_| category.set(id.clone())
                    >
                        <span class="cat-emoji">{c.icon.clone()}</span>
                        <span class="cat-name">{c.name.clone()}</span>
                    </button>
                }
            })
            .collect_view()
    };

    view! {
        <div class="overlay">
            <div class="editor">
                <header class="topbar">
                    <button class="link" on:click=move |_| close()>"ຍົກເລີກ"</button>
                    <h2>{if is_edit { "ແກ້ໄຂລາຍການ" } else { "ເພີ່ມລາຍການ" }}</h2>
                    <button class="pill" on:click=move |_| save.run(())>"ບັນທຶກ"</button>
                </header>

                <div class="segmented">
                    <button class="seg-exp" class:on=move || kind.get() == Kind::Expense on:click=move |_| set_kind(Kind::Expense)>
                        "ລາຍຈ່າຍ"
                    </button>
                    <button class="seg-inc" class:on=move || kind.get() == Kind::Income on:click=move |_| set_kind(Kind::Income)>
                        "ລາຍຮັບ"
                    </button>
                </div>

                <label class="amount-box" class:income=move || kind.get() == Kind::Income>
                    <input
                        class="amount-input"
                        inputmode="numeric"
                        pattern="[0-9]*"
                        placeholder="0"
                        autocomplete="off"
                        enterkeyhint="done"
                        aria-label="ຈຳນວນເງິນ"
                        style=move || {
                            // Grow with the number so the ₭ sign stays right next to it.
                            let chars = format_amount_input(amount.get()).chars().count().max(1);
                            format!("width: {}ch", chars as f32 + 0.3)
                        }
                        prop:value=move || format_amount_input(amount.get())
                        on:input=move |ev| amount.set(parse_amount_input(&event_target_value(&ev)))
                        on:keydown=save_on_enter
                    />
                    <span class="amount-unit">"₭"</span>
                </label>
                <div class="hint center">"ແຕະເພື່ອພິມຈຳນວນເງິນ"</div>

                <div class="cat-grid">{category_grid}</div>

                <label class="field">
                    <span>"📅 ວັນທີ"</span>
                    <span class="field-value">{move || format!("{} ›", full_date_label(date.get(), state.today.get()))}</span>
                    <input
                        type="date"
                        class="date-overlay"
                        aria-label="ວັນທີ"
                        prop:value=move || date.get().to_string()
                        on:click=move |ev| platform::show_picker(&event_target(&ev))
                        on:change=move |ev| {
                            if let Some(d) = Date::parse_iso(&event_target_value(&ev)) {
                                date.set(d);
                            }
                        }
                    />
                </label>
                <label class="field">
                    <span>"📝 ໝາຍເຫດ"</span>
                    <input
                        class="field-input"
                        placeholder="ບໍ່ບັງຄັບ"
                        maxlength="200"
                        enterkeyhint="done"
                        prop:value=move || note.get()
                        on:input=move |ev| note.set(event_target_value(&ev))
                        on:keydown=save_on_enter
                    />
                </label>

                {is_edit.then(|| view! { <button class="danger-btn" on:click=delete>"ລຶບລາຍການນີ້"</button> })}
            </div>
        </div>
    }
}

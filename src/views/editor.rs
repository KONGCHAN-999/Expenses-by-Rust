//! Full-screen form to add or edit a transaction.
//!
//! Amounts are typed on an on-screen number pad (with a "000" key), so the
//! phone keyboard never hides the categories and the Save button stays under
//! the thumb. On a computer the number keys, Backspace, Enter and Esc work too.

use expenses_core::date::Date;
use expenses_core::format::{day_label, format_amount_input};
use expenses_core::keypad::{Key, press_key};
use expenses_core::model::{Kind, Transaction, other_category_id};
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

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
    // Briefly shakes the amount when Save is pressed without one.
    let nudge = RwSignal::new(false);

    let is_income = move || kind.get() == Kind::Income;
    let set_kind = move |k: Kind| {
        if kind.get_untracked() != k {
            kind.set(k);
            category.set(first_category(k));
        }
    };
    let press = move |key: Key| amount.update(|a| *a = press_key(*a, key));
    let close = move || state.editor.set(None);
    let save = Callback::new(move |_: ()| {
        let Some(value) = amount.get_untracked().filter(|v| *v > 0) else {
            state.notify("ກະລຸນາໃສ່ຈຳນວນເງິນ");
            nudge.set(true);
            set_timeout(move || nudge.set(false), std::time::Duration::from_millis(400));
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

    // Physical keyboard: digits / Backspace / Delete edit the amount, Enter saves,
    // Esc closes. Skipped while typing in the note or another field.
    let keys = window_event_listener(ev::keydown, move |e| {
        if e.default_prevented() || e.is_composing() || e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        let target = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        let tag = target.as_ref().map(|el| el.tag_name()).unwrap_or_default();
        if matches!(tag.as_str(), "INPUT" | "TEXTAREA" | "SELECT") {
            return;
        }
        let key = match e.key().as_str() {
            "Enter" => {
                // Let Enter activate a focused button, except the number pad's own keys.
                let is_key = target.is_some_and(|el| el.class_name().split_whitespace().any(|c| c == "key"));
                let on_other_button = tag == "BUTTON" && !is_key;
                if !on_other_button {
                    e.prevent_default();
                    save.run(());
                }
                return;
            }
            "Escape" => return close(),
            "Backspace" => Key::Backspace,
            "Delete" => Key::Clear,
            k => match k.parse::<u8>() {
                Ok(d) if k.len() == 1 => Key::Digit(d),
                _ => return,
            },
        };
        e.prevent_default();
        press(key);
    });
    on_cleanup(move || keys.remove());

    let amount_text = move || match amount.get() {
        Some(v) => format_amount_input(Some(v)),
        None => "0".to_string(),
    };
    let amount_size = move || {
        let size = match amount_text().chars().count() {
            0..=9 => 50,
            10..=12 => 42,
            _ => 34,
        };
        format!("font-size: {size}px")
    };
    let selected_category = Memo::new(move |_| state.data.with(|d| d.category(&category.get()).cloned()));

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
                        style=format!("--c: {}", c.color)
                        aria-pressed=move || selected.get().to_string()
                        on:click=move |_| category.set(id.clone())
                    >
                        <span class="cat-choice-icon">{c.icon.clone()}</span>
                        <span class="cat-name">{c.name.clone()}</span>
                    </button>
                }
            })
            .collect_view()
    };

    let today = move || state.today.get();
    let yesterday = move || state.today.get().prev_day();
    let other_day = move || {
        let d = date.get();
        d != today() && d != yesterday()
    };

    let key = move |label: &'static str, k: Key, class: &'static str| {
        view! {
            <button class=format!("key {class}") on:click=move |_| press(k)>
                {label}
            </button>
        }
    };

    view! {
        <div class="overlay">
            <div class="editor" class:income=is_income>
                <header class="topbar">
                    <button class="link" on:click=move |_| close()>"ຍົກເລີກ"</button>
                    <h2>{if is_edit { "ແກ້ໄຂລາຍການ" } else { "ເພີ່ມລາຍການ" }}</h2>
                    <div class="topbar-end">
                        {is_edit
                            .then(|| {
                                view! {
                                    <button class="delete-btn" on:click=delete aria-label="ລຶບລາຍການນີ້">
                                        <TrashIcon/>
                                    </button>
                                }
                            })}
                    </div>
                </header>

                <div class="segmented">
                    <button class="seg-exp" class:on=move || !is_income() on:click=move |_| set_kind(Kind::Expense)>
                        "ລາຍຈ່າຍ"
                    </button>
                    <button class="seg-inc" class:on=is_income on:click=move |_| set_kind(Kind::Income)>
                        "ລາຍຮັບ"
                    </button>
                </div>

                <div class="amount-area">
                    <div class="amount-cat">
                        {move || {
                            selected_category
                                .get()
                                .map(|c| {
                                    view! {
                                        <span class="amount-cat-icon" style=icon_background(&c.color)>{c.icon}</span>
                                        <span>{c.name}</span>
                                    }
                                })
                        }}
                    </div>
                    <div
                        class="amount-display"
                        class:zero=move || amount.get().is_none()
                        class:nudge=move || nudge.get()
                        style=amount_size
                        role="status"
                        aria-label="ຈຳນວນເງິນ"
                    >
                        <span class="amount-value">{amount_text}</span>
                        <span class="amount-unit">"₭"</span>
                    </div>
                </div>

                <div class="cat-grid">{category_grid}</div>

                <div class="editor-bottom">
                    <div class="date-row">
                        <button
                            class="date-chip"
                            class:on=move || date.get() == today()
                            on:click=move |_| date.set(state.today.get_untracked())
                        >
                            "ມື້ນີ້"
                        </button>
                        <button
                            class="date-chip"
                            class:on=move || date.get() == yesterday()
                            on:click=move |_| date.set(state.today.get_untracked().prev_day())
                        >
                            "ມື້ວານ"
                        </button>
                        <label class="date-chip date-pick" class:on=other_day>
                            <CalendarIcon/>
                            <span>{move || if other_day() { day_label(date.get(), today()) } else { "ວັນອື່ນ".to_string() }}</span>
                            <input
                                type="date"
                                class="date-overlay"
                                aria-label="ເລືອກວັນທີ"
                                prop:value=move || date.get().to_string()
                                on:click=move |ev| platform::show_picker(&event_target(&ev))
                                on:change=move |ev| {
                                    if let Some(d) = Date::parse_iso(&event_target_value(&ev)) {
                                        date.set(d);
                                    }
                                }
                            />
                        </label>
                    </div>

                    <label class="note-field">
                        <NoteIcon/>
                        <input
                            class="note-input"
                            placeholder="ໝາຍເຫດ (ບໍ່ບັງຄັບ)"
                            maxlength="200"
                            enterkeyhint="done"
                            aria-label="ໝາຍເຫດ"
                            prop:value=move || note.get()
                            on:input=move |ev| note.set(event_target_value(&ev))
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Enter" {
                                    // "Done" just closes the keyboard; Save is on the number pad.
                                    ev.prevent_default();
                                    if let Some(input) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                                        let _ = input.blur();
                                    }
                                }
                            }
                        />
                    </label>

                    // Mouse clicks shouldn't move focus onto the keys (keeps Enter = Save on a computer).
                    <div class="keypad" on:mousedown=|ev| ev.prevent_default()>
                        {key("1", Key::Digit(1), "")}
                        {key("2", Key::Digit(2), "")}
                        {key("3", Key::Digit(3), "")}
                        <button class="key key-fn" aria-label="ລຶບຕົວເລກ" on:click=move |_| press(Key::Backspace)>
                            <BackspaceIcon/>
                        </button>
                        {key("4", Key::Digit(4), "")}
                        {key("5", Key::Digit(5), "")}
                        {key("6", Key::Digit(6), "")}
                        {key("7", Key::Digit(7), "")}
                        {key("8", Key::Digit(8), "")}
                        {key("9", Key::Digit(9), "")}
                        {key("C", Key::Clear, "key-fn")}
                        {key("0", Key::Digit(0), "")}
                        {key("000", Key::TripleZero, "key-zeros")}
                        <button
                            class="key key-save"
                            class:ready=move || amount.get().is_some()
                            on:click=move |_| save.run(())
                        >
                            <CheckIcon/>
                            <span>"ບັນທຶກ"</span>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn TrashIcon() -> impl IntoView {
    view! {
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 7h16M10 11v6M14 11v6M5 7l1 12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2l1-12M9 7V4h6v3"/>
        </svg>
    }
}

#[component]
fn BackspaceIcon() -> impl IntoView {
    view! {
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M21 5H9l-7 7 7 7h12a1 1 0 0 0 1-1V6a1 1 0 0 0-1-1zM17 9l-6 6M11 9l6 6"/>
        </svg>
    }
}

#[component]
fn CheckIcon() -> impl IntoView {
    view! {
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 12.5l4.5 4.5L19 7.5"/>
        </svg>
    }
}

#[component]
fn CalendarIcon() -> impl IntoView {
    view! {
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <rect x="3.5" y="5" width="17" height="15.5" rx="2.5"/>
            <path d="M3.5 10h17M8 3v4M16 3v4"/>
        </svg>
    }
}

#[component]
fn NoteIcon() -> impl IntoView {
    view! {
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16v4zM13.5 6.5l4 4"/>
        </svg>
    }
}

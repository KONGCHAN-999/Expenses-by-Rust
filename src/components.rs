//! Small building blocks shared by several screens.

use expenses_core::format::{day_label, signed_kip};
use expenses_core::model::{Kind, Transaction};
use leptos::prelude::*;

use crate::state::{Editor, use_app};

/// `‹  ພະຈິກ 2026  ›`
#[component]
pub fn PeriodNav(#[prop(into)] label: Signal<String>, on_prev: Callback<()>, on_next: Callback<()>) -> impl IntoView {
    view! {
        <div class="period-nav">
            <button class="arrow" aria-label="ກ່ອນໜ້າ" on:click=move |_| on_prev.run(())>"‹"</button>
            <span class="period-label">{label}</span>
            <button class="arrow" aria-label="ຖັດໄປ" on:click=move |_| on_next.run(())>"›"</button>
        </div>
    }
}

pub fn icon_background(color: &str) -> String {
    format!("background: color-mix(in srgb, {color} 20%, transparent)")
}

#[component]
pub fn CatIcon(icon: String, color: String, #[prop(optional)] big: bool) -> impl IntoView {
    view! { <span class="cat-icon" class:big=big style=icon_background(&color)>{icon}</span> }
}

pub fn amount_class(kind: Kind) -> &'static str {
    match kind {
        Kind::Expense => "amount neg",
        Kind::Income => "amount pos",
    }
}

#[derive(Clone, Copy, Default)]
struct Drag {
    x: i32,
    y: i32,
    base: i32,
    /// `Some(true)` once the gesture is clearly horizontal.
    horizontal: Option<bool>,
}

const SWIPE_OPEN: i32 = -84;

/// One transaction. Tap to edit; swipe left to reveal a delete button.
#[component]
pub fn TxRow(tx: Transaction, #[prop(optional)] show_day: bool) -> impl IntoView {
    let state = use_app();
    let category = state.data.with_untracked(|d| d.category(&tx.category_id).cloned());
    let (name, icon, color) = match category {
        Some(c) => (c.name, c.icon, c.color),
        None => ("?".to_string(), "❓".to_string(), "#94a3b8".to_string()),
    };
    // With a note: note on top, category below. Without: just the category.
    let has_note = !tx.note.trim().is_empty();
    let title = if has_note { tx.note.clone() } else { name.clone() };
    let mut sub_parts = Vec::new();
    if has_note {
        sub_parts.push(name);
    }
    if show_day {
        sub_parts.push(day_label(tx.date, state.today.get_untracked()));
    }
    let subtitle = (!sub_parts.is_empty()).then(|| sub_parts.join(" · "));

    let offset = RwSignal::new(0);
    let dragging = RwSignal::new(false);
    let drag = StoredValue::new(Drag::default());

    let id = tx.id.clone();
    let open = move |_| {
        if offset.get_untracked() != 0 {
            offset.set(0);
        } else {
            state.editor.set(Some(Editor::Edit(id.clone())));
        }
    };
    let deleted = tx.clone();
    let delete = move |_| {
        let removed = deleted.clone();
        state.data.update(|d| {
            d.delete_transaction(&removed.id);
        });
        state.notify_with_undo("ລຶບລາຍການແລ້ວ", removed);
    };

    let touch_start = move |ev: web_sys::TouchEvent| {
        if let Some(t) = ev.touches().get(0) {
            drag.set_value(Drag {
                x: t.client_x(),
                y: t.client_y(),
                base: offset.get_untracked(),
                horizontal: None,
            });
        }
    };
    let touch_move = move |ev: web_sys::TouchEvent| {
        let Some(t) = ev.touches().get(0) else { return };
        let mut d = drag.get_value();
        let (dx, dy) = (t.client_x() - d.x, t.client_y() - d.y);
        if d.horizontal.is_none() && (dx.abs() > 8 || dy.abs() > 8) {
            d.horizontal = Some(dx.abs() > dy.abs());
            drag.set_value(d);
        }
        if d.horizontal == Some(true) {
            dragging.set(true);
            offset.set((d.base + dx).clamp(SWIPE_OPEN, 0));
        }
    };
    let touch_end = move |_: web_sys::TouchEvent| {
        if drag.get_value().horizontal == Some(true) {
            dragging.set(false);
            offset.set(if offset.get_untracked() < SWIPE_OPEN / 2 { SWIPE_OPEN } else { 0 });
        }
    };

    view! {
        <div class="tx-swipe">
            <button class="tx-delete" on:click=delete tabindex="-1">"ລຶບ"</button>
            <div
                class="tx"
                class:dragging=move || dragging.get()
                style=move || format!("transform: translateX({}px)", offset.get())
                on:touchstart=touch_start
                on:touchmove=touch_move
                on:touchend=touch_end
                on:click=open
            >
                <CatIcon icon=icon color=color/>
                <div class="tx-main">
                    <div class="tx-title">{title}</div>
                    {subtitle.map(|s| view! { <div class="tx-sub">{s}</div> })}
                </div>
                <span class=amount_class(tx.kind)>{signed_kip(tx.kind, tx.amount)}</span>
            </div>
        </div>
    }
}

#[component]
pub fn Empty(icon: &'static str, title: &'static str, #[prop(optional)] hint: &'static str) -> impl IntoView {
    view! {
        <div class="empty">
            <div class="empty-icon">{icon}</div>
            <div class="empty-title">{title}</div>
            {(!hint.is_empty()).then(|| view! { <div class="empty-hint">{hint}</div> })}
        </div>
    }
}

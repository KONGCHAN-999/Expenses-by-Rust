use leptos::prelude::*;

use crate::platform;
use crate::state::{AppState, Tab, Theme, use_app};
use crate::views::editor::EditorView;
use crate::views::home::HomeView;
use crate::views::list::ListView;
use crate::views::report::ReportView;
use crate::views::settings::SettingsView;

#[component]
pub fn App() -> impl IntoView {
    let state = AppState::load();
    provide_context(state);
    state.persist();

    Effect::new(move |_| {
        let theme = state.settings.with(|s| s.theme);
        platform::set_theme_attribute(match theme {
            Theme::System => None,
            Theme::Light => Some("light"),
            Theme::Dark => Some("dark"),
        });
    });
    Effect::new(move |_| {
        state.tab.track();
        platform::scroll_to_top();
    });
    platform::on_app_visible(move || state.refresh_today());

    view! {
        <div class="app">
            <main class="screen">
                {move || match state.tab.get() {
                    Tab::Home => view! { <HomeView/> }.into_any(),
                    Tab::List => view! { <ListView/> }.into_any(),
                    Tab::Report => view! { <ReportView/> }.into_any(),
                    Tab::Settings => view! { <SettingsView/> }.into_any(),
                }}
            </main>
            <TabBar/>
            <Show when=move || state.editor.with(Option::is_some)>
                <EditorView/>
            </Show>
            <ToastView/>
        </div>
    }
}

#[component]
fn TabBar() -> impl IntoView {
    let state = use_app();
    let item = move |tab: Tab, icon: &'static str, label: &'static str| {
        view! {
            <button class="tab" class:on=move || state.tab.get() == tab on:click=move |_| state.tab.set(tab)>
                <span class="tab-icon">{icon}</span>
                <span class="tab-label">{label}</span>
            </button>
        }
    };
    view! {
        <nav class="tabbar">
            {item(Tab::Home, "🏠", "ໜ້າຫຼັກ")}
            {item(Tab::List, "📋", "ລາຍການ")}
            <button class="tab-add" aria-label="ເພີ່ມລາຍການ" on:click=move |_| state.open_new_entry()>
                "+"
            </button>
            {item(Tab::Report, "📊", "ສະຫຼຸບ")}
            {item(Tab::Settings, "⚙️", "ຕັ້ງຄ່າ")}
        </nav>
    }
}

#[component]
fn ToastView() -> impl IntoView {
    let state = use_app();
    let message = move || state.toast.with(|t| t.as_ref().map(|t| t.message.clone()).unwrap_or_default());
    let has_undo = move || state.toast.with(|t| t.as_ref().is_some_and(|t| t.undo.is_some()));
    let undo = move |_| {
        if let Some(tx) = state.toast.with_untracked(|t| t.as_ref().and_then(|t| t.undo.clone())) {
            state.data.update(|d| d.upsert_transaction(tx));
            state.notify("ກູ້ຄືນແລ້ວ");
        }
    };
    view! {
        <Show when=move || state.toast.with(Option::is_some)>
            <div class="toast" role="status">
                <span>{message}</span>
                <Show when=has_undo>
                    <button class="toast-undo" on:click=undo>"ຍົກເລີກ"</button>
                </Show>
            </div>
        </Show>
    }
}

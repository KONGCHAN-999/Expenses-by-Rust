//! Thin wrappers around browser APIs: clock, localStorage, dialogs and files.

use expenses_core::date::Date;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

pub fn now_ms() -> i64 {
    js_sys::Date::now() as i64
}

/// Today's date in the phone's local time zone.
pub fn today() -> Date {
    let d = js_sys::Date::new_0();
    Date::new(d.get_full_year() as i32, d.get_month() + 1, d.get_date()).expect("browser date is valid")
}

pub fn iso_timestamp() -> String {
    js_sys::Date::new_0().to_iso_string().into()
}

pub fn new_id() -> String {
    let random = (js_sys::Math::random() * u32::MAX as f64) as u32;
    format!("{:x}-{random:08x}", now_ms())
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn load(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

pub fn store(key: &str, value: &str) -> Result<(), ()> {
    storage().ok_or(())?.set_item(key, value).map_err(|_| ())
}

pub fn confirm(message: &str) -> bool {
    web_sys::window().and_then(|w| w.confirm_with_message(message).ok()).unwrap_or(false)
}

pub fn alert(message: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.alert_with_message(message);
    }
}

/// `None` follows the phone's light/dark setting.
pub fn set_theme_attribute(theme: Option<&str>) {
    let Some(root) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.document_element()) else {
        return;
    };
    let _ = match theme {
        Some(t) => root.set_attribute("data-theme", t),
        None => root.remove_attribute("data-theme"),
    };
}

pub fn scroll_to_top() {
    if let Some(w) = web_sys::window() {
        w.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

/// Calls `f` whenever the app comes back to the foreground.
pub fn on_app_visible(f: impl Fn() + 'static) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let cb = Closure::<dyn Fn()>::new(f);
    let _ = doc.add_event_listener_with_callback("visibilitychange", cb.as_ref().unchecked_ref());
    cb.forget(); // lives for the whole app
}

/// Opens the native date picker on desktop browsers (phones open it on tap anyway).
pub fn show_picker(input: &web_sys::HtmlInputElement) {
    if let Ok(f) = js_sys::Reflect::get(input, &"showPicker".into()) {
        if let Some(f) = f.dyn_ref::<js_sys::Function>() {
            let _ = f.call0(input);
        }
    }
}

#[wasm_bindgen]
extern "C" {
    /// Defined in index.html. Opens the share sheet on phones (so iOS can
    /// "Save to Files") and downloads elsewhere.
    /// Resolves to "shared", "downloaded" or "cancelled".
    #[wasm_bindgen(js_namespace = window, js_name = kipSaveFile)]
    fn kip_save_file(name: &str, mime: &str, content: &str) -> js_sys::Promise;
}

pub enum SaveOutcome {
    Saved,
    Cancelled,
    Failed(String),
}

/// Starts saving a file. Call this directly inside a click handler: iOS only
/// opens the share sheet in response to a tap.
pub fn start_save(name: &str, mime: &str, content: &str) -> js_sys::Promise {
    kip_save_file(name, mime, content)
}

pub async fn finish_save(promise: js_sys::Promise) -> SaveOutcome {
    match JsFuture::from(promise).await {
        Ok(v) if v.as_string().as_deref() == Some("cancelled") => SaveOutcome::Cancelled,
        Ok(_) => SaveOutcome::Saved,
        Err(e) => SaveOutcome::Failed(e.as_string().unwrap_or_else(|| format!("{e:?}"))),
    }
}

pub async fn read_file_text(file: web_sys::File) -> Result<String, String> {
    let value = JsFuture::from(file.text()).await.map_err(|e| format!("{e:?}"))?;
    value.as_string().ok_or_else(|| "not text".to_string())
}

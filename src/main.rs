mod app;
mod components;
mod platform;
mod state;
mod views;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}

//! The theme switch.
//!
//! The stored preference is applied by a tiny inline script in <head>, because
//! WebAssembly cannot be instantiated before the first paint and a reader who
//! chose dark would otherwise see a flash of light. Everything after that first
//! frame (toggling, persisting, relabelling) happens here.

use crate::{dom, Plugin};
use wasm_bindgen::JsValue;
use web_sys::{window, Document};

pub const PLUGIN: Plugin = Plugin { name: "theme", install };

const DARK_QUERY: &str = "(prefers-color-scheme: dark)";

fn install(document: &Document) -> Result<(), JsValue> {
    sync_control(document);

    if let Some(button) = dom::query(document, "[data-theme-toggle]") {
        let doc = document.clone();
        dom::listen(&button, "click", move |_| {
            let next = if effective(&doc) == "dark" { "light" } else { "dark" };
            if let Some(root) = doc.document_element() {
                let _ = root.set_attribute("data-theme", next);
            }
            store(next);
            sync_control(&doc);
        })?;
    }

    // A reader who has not chosen follows the system, which can change while
    // the page is open (an automatic evening switch, say). The icon follows by
    // itself, being CSS; the button's label has to be told.
    if let Some(query) = dom::media(DARK_QUERY) {
        let doc = document.clone();
        dom::listen(&query, "change", move |_| sync_control(&doc))?;
    }
    Ok(())
}

/// What the reader is actually looking at right now: an explicit choice if one
/// has been made, otherwise whatever the operating system asks for.
fn effective(document: &Document) -> &'static str {
    let chosen = document.document_element().and_then(|root| root.get_attribute("data-theme"));
    match chosen.as_deref() {
        Some("light") => "light",
        Some("dark") => "dark",
        _ if dom::matches(DARK_QUERY) => "dark",
        _ => "light",
    }
}

/// The button is labelled with the theme it will switch *to*, so it reads as an
/// action rather than a status.
fn sync_control(document: &Document) {
    let current = effective(document);
    let next = if current == "dark" { "Light" } else { "Dark" };

    if let Some(button) = dom::query(document, "[data-theme-toggle]") {
        let _ = button.set_attribute("aria-label", &format!("Switch to {next} theme"));
        let _ = button.set_attribute("aria-pressed", if current == "dark" { "true" } else { "false" });
    }
    if let Some(label) = dom::query(document, "[data-theme-label]") {
        label.set_text_content(Some(next));
    }
}

pub fn stored() -> Option<String> {
    window()?.local_storage().ok().flatten()?.get_item("theme").ok().flatten()
}

fn store(theme: &str) {
    if let Some(storage) = window().and_then(|win| win.local_storage().ok().flatten()) {
        let _ = storage.set_item("theme", theme);
    }
}

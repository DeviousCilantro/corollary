//! Client-side behaviour for Corollary, written in Rust and compiled to
//! WebAssembly.
//!
//! The page is complete static HTML before this module loads: every word, every
//! link and every equation is rendered by the Rust generator at build time.
//! What lives here is only what genuinely cannot be decided until the reader is
//! present — their colour theme, and what they want on their clipboard. If the
//! module fails to load, nothing is lost.

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Document, Element, Event, HtmlElement, Window};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let Some(document) = window().and_then(|win| win.document()) else {
        return Ok(());
    };

    // Lets the stylesheet distinguish an enhanced page from a plain one, should
    // that ever be needed.
    if let Some(body) = document.body() {
        let _ = body.set_attribute("data-wasm", "ready");
    }

    sync_theme_control(&document);
    install_theme_toggle(&document)?;
    watch_system_theme(&document)?;
    install_copy_buttons(&document)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Theme
//
// The stored preference is applied by a tiny inline script in <head>, because
// WebAssembly cannot be instantiated before the first paint and a reader who
// chose dark would otherwise see a flash of light. Everything after that first
// frame — toggling, persisting, relabelling — happens here.
// ---------------------------------------------------------------------------

fn install_theme_toggle(document: &Document) -> Result<(), JsValue> {
    let Some(button) = query(document, "[data-theme-toggle]") else {
        return Ok(());
    };

    let doc = document.clone();
    let on_click = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
        let next = if effective_theme(&doc) == "dark" { "light" } else { "dark" };
        set_document_theme(&doc, next);
        store_theme(next);
        sync_theme_control(&doc);
    }));

    button.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
    on_click.forget();
    Ok(())
}

/// A reader who has not chosen follows the system, which can change while the
/// page is open (an automatic evening switch, say). The icon follows by itself,
/// being CSS; the button's label has to be told.
fn watch_system_theme(document: &Document) -> Result<(), JsValue> {
    let Some(query) =
        window().and_then(|win| win.match_media("(prefers-color-scheme: dark)").ok().flatten())
    else {
        return Ok(());
    };

    let doc = document.clone();
    let on_change = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
        sync_theme_control(&doc);
    }));
    query.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())?;
    on_change.forget();
    Ok(())
}

fn set_document_theme(document: &Document, theme: &str) {
    if let Some(root) = document.document_element() {
        let _ = root.set_attribute("data-theme", theme);
    }
}

/// What the reader is actually looking at right now: an explicit choice if one
/// has been made, otherwise whatever the operating system asks for.
fn effective_theme(document: &Document) -> &'static str {
    if let Some(root) = document.document_element() {
        match root.get_attribute("data-theme").as_deref() {
            Some("light") => return "light",
            Some("dark") => return "dark",
            _ => {}
        }
    }
    if prefers_dark() {
        "dark"
    } else {
        "light"
    }
}

/// The button is labelled with the theme it will switch *to*, so it reads as an
/// action rather than a status.
fn sync_theme_control(document: &Document) {
    let current = effective_theme(document);
    let next = if current == "dark" { "Light" } else { "Dark" };

    if let Some(button) = query(document, "[data-theme-toggle]") {
        let _ = button.set_attribute("aria-label", &format!("Switch to {next} theme"));
        let _ = button
            .set_attribute("aria-pressed", if current == "dark" { "true" } else { "false" });
    }
    if let Some(label) = query(document, "[data-theme-label]") {
        label.set_text_content(Some(next));
    }
}

fn prefers_dark() -> bool {
    window()
        .and_then(|win| win.match_media("(prefers-color-scheme: dark)").ok().flatten())
        .is_some_and(|query| query.matches())
}

// ---------------------------------------------------------------------------
// Clipboard
//
// Buttons carry either the literal text to copy (`data-copy`) or a selector for
// the element holding it (`data-copy-target`).
// ---------------------------------------------------------------------------

fn install_copy_buttons(document: &Document) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-copy], [data-copy-target]")?;

    for index in 0..buttons.length() {
        let Some(button) = buttons.item(index).and_then(|n| n.dyn_into::<HtmlElement>().ok())
        else {
            continue;
        };

        let doc = document.clone();
        let element = button.clone();
        let on_click = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
            let Some(text) = copy_payload(&doc, &element) else {
                return;
            };
            copy_with_feedback(&doc, &element, &text);
        }));

        button.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
        on_click.forget();
    }
    Ok(())
}

fn copy_payload(document: &Document, button: &HtmlElement) -> Option<String> {
    if let Some(literal) = button.get_attribute("data-copy") {
        return Some(literal);
    }
    let selector = button.get_attribute("data-copy-target")?;
    let target = document.query_selector(&selector).ok().flatten()?;
    target.text_content()
}

/// Says "copied" only once the clipboard has accepted the text. If it refuses —
/// an insecure origin, a denied permission, an old browser — the text is
/// selected instead and the button names the shortcut, so the reader can copy
/// it themselves rather than being told something untrue.
fn copy_with_feedback(document: &Document, button: &HtmlElement, text: &str) {
    let Some(promise) = clipboard_write(text) else {
        fall_back(document, button);
        return;
    };

    let ok_button = button.clone();
    let on_ok = Closure::<dyn FnMut(JsValue)>::once(move |_: JsValue| {
        let done = ok_button.get_attribute("data-copy-done").unwrap_or_else(|| "copied".into());
        flash(&ok_button, &done);
    });
    let fail_doc = document.clone();
    let fail_button = button.clone();
    let on_err = Closure::<dyn FnMut(JsValue)>::once(move |_: JsValue| {
        fall_back(&fail_doc, &fail_button);
    });
    let _ = promise.then2(&on_ok, &on_err);
    // Each closure runs at most once; the few bytes they hold are not worth
    // the bookkeeping of reclaiming them.
    on_ok.forget();
    on_err.forget();
}

fn clipboard_write(text: &str) -> Option<js_sys::Promise> {
    let navigator = window()?.navigator();
    let clipboard = js_sys::Reflect::get(&navigator, &JsValue::from_str("clipboard")).ok()?;
    if clipboard.is_undefined() || clipboard.is_null() {
        return None;
    }
    Some(navigator.clipboard().write_text(text))
}

fn fall_back(document: &Document, button: &HtmlElement) {
    select_payload(document, button);
    let mac = window()
        .and_then(|win| win.navigator().user_agent().ok())
        .is_some_and(|agent| agent.contains("Mac"));
    flash(button, if mac { "\u{2318}C" } else { "Ctrl+C" });
}

/// Selects what the button would have copied: the element it targets, or the
/// email address beside it.
fn select_payload(document: &Document, button: &HtmlElement) {
    let target = match button.get_attribute("data-copy-target") {
        Some(selector) => document.query_selector(&selector).ok().flatten(),
        None => button.previous_element_sibling(),
    };
    let (Some(target), Some(selection)) =
        (target, window().and_then(|win| win.get_selection().ok().flatten()))
    else {
        return;
    };
    let _ = selection.select_all_children(&target);
}

/// Swap the button's label for a short message, then put it back.
fn flash(button: &HtmlElement, message: &str) {
    if button.has_attribute("data-copied") {
        return;
    }
    let Some(original) = button.text_content() else {
        return;
    };

    button.set_text_content(Some(message));
    let _ = button.set_attribute("data-copied", "");

    let element = button.clone();
    let restore = Closure::once_into_js(move || {
        element.set_text_content(Some(&original));
        let _ = element.remove_attribute("data-copied");
    });

    if let Some(win) = window() {
        let _ = Window::set_timeout_with_callback_and_timeout_and_arguments_0(
            &win,
            restore.unchecked_ref(),
            1400,
        );
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn query(document: &Document, selector: &str) -> Option<Element> {
    document.query_selector(selector).ok().flatten()
}

fn stored_theme() -> Option<String> {
    window()
        .and_then(|win| win.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("theme").ok().flatten())
}

fn store_theme(theme: &str) {
    if let Some(storage) = window().and_then(|win| win.local_storage().ok().flatten()) {
        let _ = storage.set_item("theme", theme);
    }
}

/// Exposed for completeness: the inline head script is the one that consumes
/// the stored value before first paint.
#[wasm_bindgen]
pub fn preferred_theme() -> Option<String> {
    stored_theme()
}

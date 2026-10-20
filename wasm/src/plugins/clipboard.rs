//! Copy buttons.
//!
//! Buttons carry either the literal text to copy (`data-copy`) or a selector for
//! the element holding it (`data-copy-target`).

use crate::{dom, Plugin};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, Document, HtmlElement, Window};

pub const PLUGIN: Plugin = Plugin { name: "clipboard", install };

fn install(document: &Document) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-copy], [data-copy-target]")?;
    for index in 0..buttons.length() {
        let Some(button) = buttons.item(index).and_then(|n| n.dyn_into::<HtmlElement>().ok())
        else {
            continue;
        };
        let doc = document.clone();
        let element = button.clone();
        dom::listen(&button, "click", move |_| {
            if let Some(text) = payload(&doc, &element) {
                copy_with_feedback(&doc, &element, &text);
            }
        })?;
    }
    Ok(())
}

fn payload(document: &Document, button: &HtmlElement) -> Option<String> {
    if let Some(literal) = button.get_attribute("data-copy") {
        return Some(literal);
    }
    let selector = button.get_attribute("data-copy-target")?;
    dom::query(document, &selector)?.text_content()
}

/// Says "copied" only once the clipboard has accepted the text. If it refuses
/// (an insecure origin, a denied permission, an old browser) the text is
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
        Some(selector) => dom::query(document, &selector),
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

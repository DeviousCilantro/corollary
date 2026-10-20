//! Small helpers shared by the plugins.

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, Document, Element, Event, EventTarget, MediaQueryList};

pub fn document() -> Option<Document> {
    window()?.document()
}

pub fn query(document: &Document, selector: &str) -> Option<Element> {
    document.query_selector(selector).ok().flatten()
}

pub fn media(query: &str) -> Option<MediaQueryList> {
    window()?.match_media(query).ok().flatten()
}

pub fn matches(query: &str) -> bool {
    media(query).is_some_and(|list| list.matches())
}

/// Calls `handler` on every `event` at `target`, for the life of the page.
pub fn listen(
    target: &EventTarget,
    event: &str,
    handler: impl FnMut(Event) + 'static,
) -> Result<(), JsValue> {
    let closure = Closure::<dyn FnMut(Event)>::wrap(Box::new(handler));
    target.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())?;
    // Listeners live as long as the page; the closure is never reclaimed.
    closure.forget();
    Ok(())
}

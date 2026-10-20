//! Client-side behaviour for Corollary, written in Rust and compiled to
//! WebAssembly.
//!
//! The page is complete static HTML before this module loads: every word, every
//! link and every equation is rendered by the Rust generator at build time.
//! What lives here is only what genuinely cannot be decided until the reader is
//! present: their colour theme, what they want on their clipboard, and the slow
//! drift of the headings' gradient. If the module fails to load, nothing is
//! lost; the headings simply hold still.
//!
//! Each behaviour is a [`Plugin`] in its own module under `plugins/`. The
//! generator names the enabled ones in the page (`<html data-plugins="…">`,
//! from `[plugins]` in content/site.toml), and only those are installed. A page
//! that names none, such as one built before the list existed, gets them all.

mod dom;
mod plugins;

use wasm_bindgen::prelude::*;
use web_sys::Document;

/// One self-contained piece of behaviour: a name the page can switch on or off,
/// and the function that wires it into the document.
pub struct Plugin {
    pub name: &'static str,
    pub install: fn(&Document) -> Result<(), JsValue>,
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let Some(document) = dom::document() else {
        return Ok(());
    };

    // Lets the stylesheet distinguish an enhanced page from a plain one.
    if let Some(body) = document.body() {
        let _ = body.set_attribute("data-wasm", "ready");
    }

    let enabled = document.document_element().and_then(|root| root.get_attribute("data-plugins"));
    for plugin in plugins::ALL {
        let on = enabled
            .as_deref()
            .map_or(true, |list| list.split_whitespace().any(|name| name == plugin.name));
        if on {
            // One plugin failing must not keep the others from installing.
            let _ = (plugin.install)(&document);
        }
    }
    Ok(())
}

/// Exposed for completeness: the inline head script is the one that consumes
/// the stored value before first paint.
#[wasm_bindgen]
pub fn preferred_theme() -> Option<String> {
    plugins::theme::stored()
}
